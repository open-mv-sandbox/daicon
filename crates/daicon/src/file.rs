use std::{collections::BTreeSet, io::Write, mem::size_of};

use anyhow::{Context as _, Error};
use bytemuck::{bytes_of, Zeroable};
use daicon_types::{Entry, Header};
use ptero_file::{FileAction, FileMessage, ReadResult, WriteLocation, WriteResult};
use stewart::{ActorId, Addr, State, System, SystemId, SystemOptions, World};
use stewart_utils::{map, map_once, when};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{
    cache::CachedTable,
    set::{start_set_task, SetTaskSystem},
    SourceAction, SourceMessage,
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
    let id = world.create(parent)?;
    let addr = Addr::new(id);

    let system = world.register(FileSourceSystem, id, SystemOptions::default());
    let set_task = world.register(SetTaskSystem, id, SystemOptions::default());

    let source = map(world, Some(id), addr, Message::SourceMessage)?;
    let mut table = None;

    // TODO: this is also the validation step, respond if we correctly validated
    match mode {
        OpenMode::ReadWrite => {
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
            // Start writing immediately at the given offset
            let create_table = CachedTable::new(0, 256);
            let mut data = Vec::new();

            // Write the header
            let (header, _) = create_table.create_header();
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
                on_result: when(world, Some(id), SystemOptions::default(), |_, _, _| {
                    Ok(false)
                })?,
            };
            let message = FileMessage {
                id: Uuid::new_v4(),
                action,
            };
            world.send(file, message);

            // Store the table
            table = Some(create_table);
        }
    }

    // Start the root manager actor
    let instance = FileSource {
        id,
        set_task,

        write_header_result: map(world, Some(id), addr, Message::WriteHeaderResult)?,
        file,
        table,

        get_tasks: Vec::new(),
        pending_slots: Vec::new(),
    };
    world.start(id, system, instance)?;

    Ok(source)
}

pub enum OpenMode {
    ReadWrite,
    Create,
}

struct FileSourceSystem;

impl System for FileSourceSystem {
    type Instance = FileSource;
    type Message = Message;

    #[instrument("file-source", skip_all)]
    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        let mut changed = BTreeSet::new();

        while let Some((actor, message)) = state.next() {
            let instance = state.get_mut(actor).context("failed to get instance")?;

            match message {
                Message::SourceMessage(message) => {
                    instance.on_source_message(world, message)?;
                }
                Message::ReadTableResult(result) => {
                    instance.on_read_table(result)?;
                }
                Message::WriteHeaderResult(result) => {
                    instance.on_write(result)?;
                }
            }

            changed.insert(actor);
        }

        // Check if we can resolve any get requests
        for actor in changed {
            let instance = state.get_mut(actor).context("failed to get instance")?;
            instance.check_pending(world);
        }

        Ok(())
    }
}

struct FileSource {
    id: ActorId,
    set_task: SystemId,

    write_header_result: Addr<WriteResult>,
    file: Addr<FileMessage>,
    table: Option<CachedTable>,

    pending_slots: Vec<Addr<u32>>,

    // TODO: Stateful temporary tasks should also be actors, most of set already is
    get_tasks: Vec<(Uuid, Addr<ReadResult>)>,
}

impl FileSource {
    fn on_source_message(
        &mut self,
        world: &mut World,
        message: SourceMessage,
    ) -> Result<(), Error> {
        match message.action {
            SourceAction::Get { id, on_result } => {
                event!(Level::INFO, ?id, "received get");
                self.get_tasks.push((id, on_result));
            }
            SourceAction::Set {
                id,
                data,
                on_result,
            } => {
                event!(Level::INFO, ?id, bytes = data.len(), "received set");

                let addr = start_set_task(
                    world,
                    Some(self.id),
                    self.set_task,
                    self.file,
                    id,
                    data,
                    on_result,
                )?;
                self.pending_slots.push(addr);
            }
        }

        Ok(())
    }

    fn on_read_table(&mut self, result: ReadResult) -> Result<(), Error> {
        let table = CachedTable::read(result.offset as u32, result.data)?;
        self.table = Some(table);

        // TODO: Follow additional headers

        Ok(())
    }

    fn on_write(&mut self, _result: WriteResult) -> Result<(), Error> {
        // TODO: Report back entry valid once it falls in the header's valid range

        Ok(())
    }

    fn check_pending(&mut self, world: &mut World) {
        let table = if let Some(table) = self.table.as_mut() {
            table
        } else {
            return;
        };

        // Resolve pending gets
        self.get_tasks
            .retain(|(id, on_result)| !try_read_data(world, self.file, table, *id, *on_result));

        // Resolve pending sets
        self.pending_slots.retain(|on_result| {
            !try_allocate_slot(
                world,
                self.write_header_result,
                self.file,
                table,
                *on_result,
            )
        });

        // TODO: Reply failure if we ran out of tables to read, and couldn't find it
        // TODO: Allocate new tables if we ran out of free spaces
    }
}

enum Message {
    SourceMessage(SourceMessage),
    ReadTableResult(ReadResult),
    WriteHeaderResult(WriteResult),
}

fn try_read_data(
    world: &mut World,
    file: Addr<FileMessage>,
    table: &mut CachedTable,
    id: Uuid,
    on_result: Addr<ReadResult>,
) -> bool {
    let entry = if let Some(entry) = table.find(id) {
        entry
    } else {
        return false;
    };

    event!(
        Level::INFO,
        id = ?entry.id(),
        "found entry"
    );

    // We found a matching entry, start the read to fetch the inner data
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Read {
            offset: entry.offset() as u64,
            size: entry.size() as u64,
            on_result,
        },
    };
    world.send(file, message);

    true
}

fn try_allocate_slot(
    world: &mut World,
    write_header_result: Addr<WriteResult>,
    file: Addr<FileMessage>,
    table: &mut CachedTable,
    on_result: Addr<u32>,
) -> bool {
    let index = if let Some(index) = table.try_allocate() {
        index
    } else {
        return false;
    };

    // Reply that we've found a slot
    let offset = table.entry_offset(index);
    world.send(on_result, offset);

    // Write the new header with the updated valid count
    // TODO: Wait until the task tells us to validate
    // TODO: Get the entry back from the task, currently the cache is wrong
    table.mark_valid(index, Entry::default());
    let (header, offset) = table.create_header();
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Write {
            location: WriteLocation::Offset(offset as u64),
            data: bytes_of(&header).to_owned(),
            on_result: write_header_result,
        },
    };
    world.send(file, message);

    true
}
