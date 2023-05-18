use anyhow::Error;
use bytemuck::{bytes_of, Zeroable};
use daicon_types::Entry;
use ptero_file::{FileAction, FileMessage, WriteLocation, WriteResult};
use stewart::{Actor, ActorId, Addr, Options, State, World};
use stewart_utils::{map, map_once};
use tracing::{event, instrument, Level};
use uuid::Uuid;

#[instrument("set-task", skip_all)]
pub fn start_set_task(
    world: &mut World,
    parent: Option<ActorId>,
    file: Addr<FileMessage>,
    id: Uuid,
    data: Vec<u8>,
    on_result: Addr<()>,
) -> Result<Addr<u32>, Error> {
    let actor_id = world.create(parent, Options::default())?;
    let addr = Addr::new(actor_id);

    // Start the append immediately
    let size = data.len() as u32;
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Write {
            location: WriteLocation::Append,
            data,
            on_result: map(world, Some(actor_id), addr, Message::AppendResult)?,
        },
    };
    world.send(file, message);

    // Create the actor for tracking state of writing
    let mut entry = Entry::zeroed();
    entry.set_id(id);
    entry.set_size(size);
    let task = SetTask {
        id: actor_id,

        file,
        on_result,

        entry_offset: None,
        data_offset: None,
        entry,
    };
    world.start(actor_id, task)?;

    Ok(map_once(world, Some(actor_id), addr, Message::Slot)?)
}

struct SetTask {
    id: ActorId,

    file: Addr<FileMessage>,
    on_result: Addr<()>,

    entry_offset: Option<u32>,
    data_offset: Option<u32>,
    entry: Entry,
}

impl Actor for SetTask {
    type Message = Message;

    #[instrument("set-task", skip_all)]
    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                Message::Slot(offset) => {
                    self.entry_offset = Some(offset);
                }
                Message::AppendResult(message) => {
                    self.data_offset = Some(message.offset as u32);
                }
                Message::EntryResult(_message) => {
                    // TODO: Report valid back to service, and only after that's flushed report result

                    event!(Level::DEBUG, "success, sending result");

                    world.send(self.on_result, ());
                    world.stop(self.id)?;
                    return Ok(());
                }
            }

            // If we have the metadata we need to write the data, do it
            // TODO: Handle in bulk
            if let (Some(entry_offset), Some(data_offset)) = (self.entry_offset, self.data_offset) {
                event!(Level::DEBUG, "writing table entry");

                // Complete the entry
                self.entry.set_offset(data_offset);

                // Write the entry to the slot we got
                let message = FileMessage {
                    id: Uuid::new_v4(),
                    action: FileAction::Write {
                        location: WriteLocation::Offset(entry_offset as u64),
                        data: bytes_of(&self.entry).to_owned(),
                        on_result: map_once(
                            world,
                            Some(self.id),
                            Addr::new(self.id),
                            Message::EntryResult,
                        )?,
                    },
                };
                world.send(self.file, message);
            }
        }

        Ok(())
    }
}

pub enum Message {
    Slot(u32),
    AppendResult(WriteResult),
    EntryResult(WriteResult),
}
