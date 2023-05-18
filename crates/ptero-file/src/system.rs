use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
};

use anyhow::{Context as _, Error};
use stewart::{ActorId, Addr, State, System, SystemId, SystemOptions, World};
use tracing::{event, instrument, Level};

use crate::{FileAction, FileMessage, ReadResult, WriteLocation, WriteResult};

#[instrument(skip_all)]
pub fn open_system_file(world: &mut World) -> Result<Addr<SystemFileServiceMessage>, Error> {
    let id = world.create(None)?;

    // Create the service system
    let system = SystemFileServiceSystem {
        system: world.register(SystemFileSystem, id, SystemOptions::default()),
    };
    let system = world.register(system, id, SystemOptions::default());

    // Start the service
    let instance = ();
    world.start(id, system, instance)?;

    Ok(Addr::new(id))
}

pub enum SystemFileServiceMessage {
    Open {
        parent: Option<ActorId>,
        path: String,
        truncate: bool,
        on_result: Addr<Addr<FileMessage>>,
    },
}

struct SystemFileServiceSystem {
    system: SystemId,
}

impl System for SystemFileServiceSystem {
    type Instance = ();
    type Message = SystemFileServiceMessage;

    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some((_id, message)) = state.next() {
            let SystemFileServiceMessage::Open {
                parent,
                path,
                truncate,
                on_result,
            } = message;

            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .truncate(truncate)
                .create(true)
                .open(path)
                .context("failed to open system file for writing")?;

            let id = world.create(parent)?;
            let instance = SystemFile { file };
            world.start(id, self.system, instance)?;

            let addr: Addr<FileMessage> = Addr::new(id);
            world.send(on_result, addr);
        }

        Ok(())
    }
}

struct SystemFileSystem;

impl System for SystemFileSystem {
    type Instance = SystemFile;
    type Message = FileMessage;

    #[instrument("system-file", skip_all)]
    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        event!(Level::INFO, "handling messages");

        while let Some((actor, message)) = state.next() {
            let instance = state.get_mut(actor).context("failed to get instance")?;

            match message.action {
                FileAction::Read {
                    offset,
                    size,
                    on_result,
                } => {
                    // TODO: Currently remaining bytes after EOF are kept zero, but maybe we want to
                    // feedback a lack of remaining bytes.

                    let mut data = vec![0u8; size as usize];

                    instance.file.seek(SeekFrom::Start(offset))?;
                    read_exact_eof(&mut instance.file, &mut data)?;

                    // Reply result
                    let result = ReadResult {
                        id: message.id,
                        offset,
                        data,
                    };
                    world.send(on_result, result);
                }
                FileAction::Write {
                    location,
                    data,
                    on_result,
                } => {
                    // Seek to given location
                    let from = match location {
                        WriteLocation::Offset(offset) => SeekFrom::Start(offset),
                        WriteLocation::Append => SeekFrom::End(0),
                    };
                    instance.file.seek(from)?;
                    let offset = instance.file.stream_position()?;

                    // Perform the write
                    instance.file.write_all(&data)?;

                    // Reply result
                    let result = WriteResult {
                        id: message.id,
                        offset,
                    };
                    world.send(on_result, result);
                }
            }
        }

        Ok(())
    }
}

struct SystemFile {
    file: File,
}

/// Copy of read_exact except allowing for EOF.
fn read_exact_eof(file: &mut File, mut buf: &mut [u8]) -> Result<(), Error> {
    while !buf.is_empty() {
        match file.read(buf) {
            Ok(0) => break,
            Ok(n) => {
                let tmp = buf;
                buf = &mut tmp[n..];
            }
            Err(error) => match error.kind() {
                ErrorKind::Interrupted => {}
                ErrorKind::UnexpectedEof => break,
                _ => return Err(error.into()),
            },
        }
    }

    Ok(())
}
