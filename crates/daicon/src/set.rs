use anyhow::{Context as _, Error};
use bytemuck::{bytes_of, Zeroable};
use daicon_types::Entry;
use ptero_file::{FileAction, FileMessage, WriteLocation, WriteResult};
use stewart::{ActorId, Addr, State, System, SystemId, World};
use stewart_utils::{map, map_once};
use tracing::{event, instrument, Level};
use uuid::Uuid;

#[instrument("set-task", skip_all)]
pub fn start_set_task(
    world: &mut World,
    parent: Option<ActorId>,
    system: SystemId,
    file: Addr<FileMessage>,
    id: Uuid,
    data: Vec<u8>,
    on_result: Addr<()>,
) -> Result<Addr<u32>, Error> {
    let actor = world.create(parent)?;
    let addr = Addr::new(actor);

    // Start the append immediately
    let size = data.len() as u32;
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Write {
            location: WriteLocation::Append,
            data,
            on_result: map(world, Some(actor), addr, Message::AppendResult)?,
        },
    };
    world.send(file, message);

    // Create the actor for tracking state of writing
    let mut entry = Entry::zeroed();
    entry.set_id(id);
    entry.set_size(size);
    let task = SetTask {
        file,
        on_result,

        entry_offset: None,
        data_offset: None,
        entry,
    };
    world.start(actor, system, task)?;

    Ok(map_once(world, Some(actor), addr, Message::Slot)?)
}

pub struct SetTaskSystem;

impl System for SetTaskSystem {
    type Instance = SetTask;
    type Message = Message;

    #[instrument("set-task", skip_all)]
    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some((actor, message)) = state.next() {
            let instance = state.get_mut(actor).context("failed to get instance")?;

            match message {
                Message::Slot(offset) => {
                    instance.entry_offset = Some(offset);
                }
                Message::AppendResult(message) => {
                    instance.data_offset = Some(message.offset as u32);
                }
                Message::EntryResult(_message) => {
                    // TODO: Report valid back to service, and only after that's flushed report result

                    event!(Level::DEBUG, "success, sending result");

                    world.send(instance.on_result, ());
                    world.stop(actor)?;
                    return Ok(());
                }
            }

            // If we have the metadata we need to write the data, do it
            // TODO: Handle in bulk
            if let (Some(entry_offset), Some(data_offset)) =
                (instance.entry_offset, instance.data_offset)
            {
                event!(Level::DEBUG, "writing table entry");

                // Complete the entry
                instance.entry.set_offset(data_offset);

                // Write the entry to the slot we got
                let message = FileMessage {
                    id: Uuid::new_v4(),
                    action: FileAction::Write {
                        location: WriteLocation::Offset(entry_offset as u64),
                        data: bytes_of(&instance.entry).to_owned(),
                        on_result: map_once(
                            world,
                            Some(actor),
                            Addr::new(actor),
                            Message::EntryResult,
                        )?,
                    },
                };
                world.send(instance.file, message);
            }
        }

        Ok(())
    }
}

pub struct SetTask {
    file: Addr<FileMessage>,
    on_result: Addr<()>,

    entry_offset: Option<u32>,
    data_offset: Option<u32>,
    entry: Entry,
}

pub enum Message {
    Slot(u32),
    AppendResult(WriteResult),
    EntryResult(WriteResult),
}
