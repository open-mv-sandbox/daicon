use std::{collections::HashMap, io::Write, mem::size_of};

use anyhow::{Context, Error};
use bytemuck::{bytes_of, Zeroable};
use daicon_types::{Entry, Header};
use stewart::{Actor, ActorId, Addr, Options, State, World};
use stewart_utils::{map, map_once, when};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{
    file::{FileAction, FileMessage, ReadResult, WriteLocation, WriteResult},
    source::{
        state::{SourceState, TableState},
        SourceAction, SourceMessage,
    },
};

/// Open a file as a daicon source.
///
/// A "source" returns a file from UUIDs. A "file source" uses a file as a source.
#[instrument("file-source", skip_all)]
pub fn open_file_source(
    world: &mut World,
    parent: Option<ActorId>,
    file: Addr<FileMessage>,
    mode: OpenMode,
) -> Result<Addr<SourceMessage>, Error> {
    let id = world.create(parent, Options::default())?;
    let addr = Addr::new(id);

    let source = map(world, Some(id), addr, Message::SourceMessage)?;
    let mut state = SourceState::new();

    // TODO: this is also the validation step, respond if we correctly validated
    match mode {
        OpenMode::ReadWrite => {
            event!(Level::DEBUG, "reading first header");

            // Immediately start table read
            let size = (size_of::<Header>() + (size_of::<Entry>() * 256)) as u64;
            let message = FileMessage {
                id: Uuid::new_v4(),
                action: FileAction::Read {
                    offset: 0,
                    size,
                    on_result: map_once(world, Some(id), addr, Message::ReadTableResult)?,
                },
            };
            world.send(file, message);
        }
        OpenMode::Create => {
            event!(Level::DEBUG, "writing stub");

            // Start writing immediately at the given offset
            let create_table = TableState::empty(0, 256);
            let mut data = Vec::new();

            // Write the header
            let mut header = Header::default();
            create_table.write_header(&mut header);
            data.write_all(bytes_of(&header))?;

            // Write empty entries
            for _ in 0..256 {
                let entry = Entry::zeroed();
                data.write_all(bytes_of(&entry))?;
            }

            // Send to file for writing
            let action = FileAction::Write {
                location: WriteLocation::Offset(0),
                data,
                on_result: when(world, Some(id), Options::default(), |_, _| Ok(false))?,
            };
            let message = FileMessage {
                id: Uuid::new_v4(),
                action,
            };
            world.send(file, message);

            // Store the table
            state.set_table(create_table);
        }
    }

    // Start the root manager actor
    let instance = FileSource {
        id,

        file,
        state,

        get_tasks: HashMap::new(),
        set_tasks: HashMap::new(),
    };
    world.start(id, instance)?;

    Ok(source)
}

pub enum OpenMode {
    ReadWrite,
    Create,
}

struct FileSource {
    id: ActorId,

    file: Addr<FileMessage>,
    state: SourceState,

    // Ongoing tracked requests
    get_tasks: HashMap<Uuid, GetTask>,
    set_tasks: HashMap<Uuid, SetTask>,
}

struct GetTask {
    id: u32,
    on_result: Addr<ReadResult>,
}

struct SetTask {
    id: u32,
    offset: Option<u64>,
    size: u32,
    on_result: Addr<()>,
}

impl Actor for FileSource {
    type Message = Message;

    #[instrument("file-source", skip_all)]
    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                Message::SourceMessage(message) => {
                    self.on_source_message(world, message)?;
                }
                Message::ReadTableResult(result) => {
                    self.on_read_table_result(result)?;
                }
                Message::SetWriteDataResult(result) => {
                    self.on_set_write_data_result(result)?;
                }
            }
        }

        // Check if we can resolve any get requests
        self.update_tasks(world);

        Ok(())
    }
}

impl FileSource {
    fn on_source_message(
        &mut self,
        world: &mut World,
        message: SourceMessage,
    ) -> Result<(), Error> {
        match message.action {
            SourceAction::Get { id, on_result } => {
                self.on_get(message.id, id, on_result);
            }
            SourceAction::Set {
                id,
                data,
                on_result,
            } => {
                self.on_set(world, message.id, id, data, on_result)?;
            }
        }

        Ok(())
    }

    fn on_read_table_result(&mut self, result: ReadResult) -> Result<(), Error> {
        let table = TableState::read(result.offset, result.data)?;
        self.state.set_table(table);

        // TODO: Follow additional headers

        Ok(())
    }

    fn on_get(&mut self, action_id: Uuid, id: u32, on_result: Addr<ReadResult>) {
        event!(Level::INFO, "received get {:#010x}", id);

        // Track the get task, the task update step will see what to do
        let task = GetTask { id, on_result };
        self.get_tasks.insert(action_id, task);
    }

    fn on_set(
        &mut self,
        world: &mut World,
        action_id: Uuid,
        id: u32,
        data: Vec<u8>,
        on_result: Addr<()>,
    ) -> Result<(), Error> {
        event!(
            Level::INFO,
            id = ?action_id,
            bytes = data.len(),
            "received set {:#010x}",
            id
        );

        // TODO: Check we have room and are in range of the table, before assuming we can append

        // Append the data to the file
        let size = data.len() as u32;
        let message = FileMessage {
            id: action_id,
            action: FileAction::Write {
                location: WriteLocation::Append,
                data,
                on_result: map(
                    world,
                    Some(self.id),
                    Addr::new(self.id),
                    Message::SetWriteDataResult,
                )?,
            },
        };
        world.send(self.file, message);

        // Track the request
        let task = SetTask {
            id,
            offset: None,
            size,
            on_result,
        };
        self.set_tasks.insert(action_id, task);

        Ok(())
    }

    fn on_set_write_data_result(&mut self, result: WriteResult) -> Result<(), Error> {
        event!(Level::DEBUG, id = ?result.id, "received data write result");

        // Track the location where we've written the data to
        let task = self
            .set_tasks
            .get_mut(&result.id)
            .context("failed to get pending set task")?;
        task.offset = Some(result.offset);

        Ok(())
    }

    fn update_tasks(&mut self, world: &mut World) {
        // TODO: Encapsulate tasks, those can just be simple self-contained types, the actor only
        //  has to route to the tasks.

        // For gets, if we know where the data is, start a read request and then we're done
        self.get_tasks.retain(|action_id, task| {
            let entry = if let Some(entry) = self.state.find(task.id) {
                entry
            } else {
                return true;
            };

            // We found a matching entry, start the read to fetch the inner data
            event!(Level::DEBUG, ?entry, "reading data for entry");
            let message = FileMessage {
                id: *action_id,
                action: FileAction::Read {
                    offset: entry.offset() as u64,
                    size: entry.size() as u64,
                    on_result: task.on_result,
                },
            };
            world.send(self.file, message);

            false
        });

        // TODO: Use the state instead of table directly
        let table = if let Some(table) = self.state.table_mut() {
            table
        } else {
            return;
        };

        // For sets, if we know where the data is, start a write request and then we're done
        self.set_tasks.retain(|_, task| {
            let offset = if let Some(offset) = task.offset {
                offset
            } else {
                return true;
            };

            let result = try_push_write_entry(
                world,
                self.id,
                self.file,
                table,
                task.id,
                offset,
                task.size,
                task.on_result,
            );
            match result {
                Ok(value) => !value,
                Err(error) => {
                    event!(Level::ERROR, ?error, "error");
                    false
                }
            }
        });
    }
}

enum Message {
    SourceMessage(SourceMessage),
    ReadTableResult(ReadResult),
    SetWriteDataResult(WriteResult),
}

fn try_push_write_entry(
    world: &mut World,
    actor_id: ActorId,
    file: Addr<FileMessage>,
    table: &mut TableState,
    id: u32,
    offset: u64,
    size: u32,
    on_result: Addr<()>,
) -> Result<bool, Error> {
    let (index, entry) = match table.try_push(id, offset, size) {
        Ok(index) => index,
        Err(error) => {
            event!(Level::DEBUG, ?error, "error while attempting push");
            return Ok(false);
        }
    };

    event!(Level::DEBUG, ?entry, "writing table update");
    let entry_offset = table.entry_offset(index);

    // TODO: Better encapsulate reading/writing

    // Write the entry to the slot we got
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Write {
            location: WriteLocation::Offset(entry_offset as u64),
            data: bytes_of(&entry).to_owned(),
            on_result: when(world, Some(actor_id), Options::default(), |_, _| Ok(false))?,
        },
    };
    world.send(file, message);

    // Write the new header with the updated valid count
    let mut header = Header::default();
    table.write_header(&mut header);
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Write {
            location: WriteLocation::Offset(table.offset() as u64),
            data: bytes_of(&header).to_owned(),
            on_result: when(world, Some(actor_id), Options::default(), |_, _| Ok(false))?,
        },
    };
    world.send(file, message);

    // Send out that we're done
    world.send(on_result, ());

    Ok(true)
}
