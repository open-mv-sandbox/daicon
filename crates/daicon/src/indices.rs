use std::{
    collections::HashMap,
    io::Cursor,
    io::{Read, Write},
    mem::size_of,
};

use anyhow::Error;
use bytemuck::{bytes_of, bytes_of_mut, cast_slice_mut};
use daicon_types::{Entry, Header};
use stewart::{Actor, ActorId, Addr, Options, State, World};
use stewart_utils::{map, map_once, when};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{
    file::{FileAction, FileMessage, ReadResult, WriteLocation},
    OpenMode,
};

pub fn start(
    world: &mut World,
    parent: Option<ActorId>,
    file: Addr<FileMessage>,
    mode: OpenMode,
) -> Result<Addr<IndicesMessage>, Error> {
    event!(Level::DEBUG, "starting indices service");

    let id = world.create(parent, Options::default())?;
    let addr = Addr::new(id);

    let mut tables = Vec::new();

    // TODO: This is also the validation step, respond if we correctly validated
    match mode {
        OpenMode::ReadWrite => {
            read_table(world, file, id)?;
        }
        OpenMode::Create => {
            // Start writing immediately at the given offset
            let mut header = Header::default();
            header.set_capacity(256);
            let table = Table {
                location: 0,
                offset: 0,
                capacity: 256,
                entries: Vec::new(),
            };

            write_table(world, file, id, &table)?;

            // Track the table we just wrote
            tables.push(table);
        }
    }

    // Start the actor
    let actor = Indices {
        id,
        file,

        tables: Vec::new(),
        get_tasks: HashMap::new(),
        set_tasks: HashMap::new(),
    };
    world.start(id, actor)?;

    let action_addr = map(world, Some(id), addr, Message::Action)?;
    Ok(action_addr)
}

pub struct IndicesMessage {
    pub id: Uuid,
    pub action: IndicesAction,
}

pub enum IndicesAction {
    Get {
        id: u32,
        on_result: Addr<(Uuid, u64, u32)>,
    },
    Set {
        id: u32,
        offset: u64,
        size: u32,
        on_result: Addr<()>,
    },
}

struct Indices {
    id: ActorId,
    file: Addr<FileMessage>,

    tables: Vec<Table>,
    get_tasks: HashMap<Uuid, (u32, Addr<(Uuid, u64, u32)>)>,
    set_tasks: HashMap<Uuid, (u32, u64, u32, Addr<()>)>,
}

impl Actor for Indices {
    type Message = Message;

    #[instrument("indices", skip_all)]
    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                Message::Action(message) => self.on_action(message),
                Message::ReadResult(message) => self.on_read_result(message)?,
            }
        }

        self.update_tasks(world);

        Ok(())
    }
}

impl Indices {
    fn on_action(&mut self, message: IndicesMessage) {
        match message.action {
            IndicesAction::Get { id, on_result } => {
                event!(Level::DEBUG, "received get {:#010x}", id);
                self.get_tasks.insert(message.id, (id, on_result));
            }
            IndicesAction::Set {
                id,
                offset,
                size,
                on_result,
            } => {
                event!(Level::DEBUG, "received set {:#010x}", id);
                self.set_tasks
                    .insert(message.id, (id, offset, size, on_result));
            }
        }
    }

    fn on_read_result(&mut self, message: ReadResult) -> Result<(), Error> {
        event!(Level::DEBUG, "received read result");

        let table = parse_table(message)?;
        self.tables.push(table);

        Ok(())
    }

    fn update_tasks(&mut self, world: &mut World) {
        // Resolve gets we can resolve
        self.get_tasks
            .retain(|id, task| match find(&self.tables, task.0) {
                Some(value) => {
                    event!(Level::DEBUG, "found entry {:#010x}", task.0);
                    world.send(task.1, (*id, value.0, value.1));
                    false
                }
                None => true,
            });

        // Resolve writes we can resolve
        self.set_tasks.retain(|_, task| {
            // Find a table with an empty slot
            let table = if let Some(table) = self.tables.first_mut() {
                table
            } else {
                return true;
            };

            event!(Level::DEBUG, "setting entry {:#010x}", task.0);

            // TODO: Allocate a new table if we've read all and we can't find a slot
            if task.1 < table.offset {
                event!(Level::ERROR, "cannot insert entry, case not implemented");
                return false;
            }
            let relative = task.1 - table.offset;
            if relative > u32::MAX as u64 || table.entries.len() >= table.capacity as usize {
                event!(Level::ERROR, "cannot insert entry, case not implemented");
                return false;
            }

            // Insert the new entry
            let mut entry = Entry::default();
            entry.set_id(task.0);
            entry.set_offset(relative as u32);
            entry.set_size(task.2);
            table.entries.push(entry);

            // Flush write the table
            write_table(world, self.file, self.id, table).unwrap();

            // Report success
            // TODO: Wait for write to report back
            world.send(task.3, ());

            false
        });
    }
}

struct Table {
    location: u64,
    offset: u64,
    capacity: u16,
    entries: Vec<Entry>,
}

impl Table {
    fn find(&self, id: u32) -> Option<(u64, u32)> {
        self.entries
            .iter()
            .find(|entry| entry.id() == id)
            .map(|entry| {
                let offset = entry.offset() as u64 + self.offset;
                (offset, entry.size())
            })
    }
}

enum Message {
    Action(IndicesMessage),
    ReadResult(ReadResult),
}

fn find(tables: &[Table], id: u32) -> Option<(u64, u32)> {
    tables.iter().find_map(|table| table.find(id))
}

fn read_table(world: &mut World, file: Addr<FileMessage>, id: ActorId) -> Result<(), Error> {
    // Start reading the first header
    let size = (size_of::<Header>() + (size_of::<Entry>() * 256)) as u64;
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Read {
            offset: 0,
            size,
            on_result: map_once(world, Some(id), Addr::new(id), Message::ReadResult)?,
        },
    };
    world.send(file, message);

    Ok(())
}

fn parse_table(result: ReadResult) -> Result<Table, Error> {
    // TODO: Retry if the table's valid data is larger than what we've read.
    //  This happens if the read length heuristic is too small, we need to retry then.

    let mut data = Cursor::new(result.data);

    // Read the header
    let mut header = Header::default();
    data.read_exact(bytes_of_mut(&mut header))?;

    // Read entries
    let mut entries = vec![Entry::default(); header.valid() as usize];
    data.read_exact(cast_slice_mut(&mut entries))?;

    let table = Table {
        location: result.offset,
        offset: header.offset(),
        capacity: header.capacity(),
        entries,
    };
    Ok(table)
}

fn write_table(
    world: &mut World,
    file: Addr<FileMessage>,
    id: ActorId,
    table: &Table,
) -> Result<(), Error> {
    let mut data = Vec::new();

    // Write the header
    let mut header = Header::default();
    header.set_offset(table.offset);
    header.set_capacity(table.capacity);
    header.set_valid(table.entries.len() as u16);
    data.write_all(bytes_of(&header))?;

    // Write entries
    for entry in &table.entries {
        data.write_all(bytes_of(entry))?;
    }

    // Pad with empty entries
    let empty = Entry::default();
    for _ in 0..(table.capacity as usize - table.entries.len()) {
        data.write_all(bytes_of(&empty))?;
    }

    // Send to file for writing
    let action = FileAction::Write {
        location: WriteLocation::Offset(table.location),
        data,
        on_result: when(world, Some(id), Options::default(), |_, _| Ok(false))?,
    };
    let message = FileMessage {
        id: Uuid::new_v4(),
        action,
    };
    world.send(file, message);

    Ok(())
}
