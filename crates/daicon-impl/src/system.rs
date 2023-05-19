use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
};

use anyhow::{Context, Error};
use daicon::io::{FileAction, FileMessage, ReadResult, WriteLocation, WriteResult};
use stewart::{Actor, ActorId, Addr, Options, State, World};
use tracing::{event, instrument, Level};

#[instrument(skip_all)]
pub fn open_system_file(
    world: &mut World,
    parent: Option<ActorId>,
    path: String,
    truncate: bool,
) -> Result<Addr<FileMessage>, Error> {
    let id = world.create(parent, Options::default())?;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(truncate)
        .create(true)
        .open(path)
        .context("failed to open system file for writing")?;

    let actor = SystemFile { file };
    world.start(id, actor)?;

    let addr = Addr::new(id);
    Ok(addr)
}

struct SystemFile {
    file: File,
}

impl Actor for SystemFile {
    type Message = FileMessage;

    #[instrument("system-file", skip_all)]
    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        event!(Level::INFO, "handling messages");

        while let Some(message) = state.next() {
            match message.action {
                FileAction::Read {
                    offset,
                    size,
                    on_result,
                } => {
                    // TODO: Currently remaining bytes after EOF are kept zero, but maybe we want to
                    // feedback a lack of remaining bytes.

                    let mut data = vec![0u8; size as usize];

                    self.file.seek(SeekFrom::Start(offset))?;
                    read_exact_eof(&mut self.file, &mut data)?;

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
                    self.file.seek(from)?;
                    let offset = self.file.stream_position()?;

                    // Perform the write
                    self.file.write_all(&data)?;

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
