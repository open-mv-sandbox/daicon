use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
};

use anyhow::{Context as _, Error};
use daicon::protocol::file;
use stewart::{Actor, Context, Handler, Id, World};
use tracing::{event, instrument, Level};

#[instrument(skip_all)]
pub fn open_system_file(
    world: &mut World,
    id: Id,
    path: String,
    truncate: bool,
) -> Result<Handler<file::Request>, Error> {
    event!(Level::INFO, "opening");

    let id = world.create(id, "daicon-system-file")?;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(truncate)
        .create(true)
        .open(path)
        .context("failed to open system file for writing")?;

    let actor = SystemFile { file };
    world.start(id, actor)?;

    Ok(Handler::to(id))
}

struct SystemFile {
    file: File,
}

impl Actor for SystemFile {
    type Message = file::Request;

    fn process(&mut self, world: &mut World, mut cx: Context<Self>) -> Result<(), Error> {
        println!("WOKEN UP");

        while let Some(message) = cx.next() {
            match message.action {
                file::Action::Read(action) => {
                    event!(Level::DEBUG, "reading from file");

                    // TODO: Currently remaining bytes after EOF are kept zero, but maybe we want to
                    // feedback a lack of remaining bytes.

                    let mut data = vec![0u8; action.size as usize];

                    self.file.seek(SeekFrom::Start(action.offset))?;
                    read_exact_eof(&mut self.file, &mut data)?;

                    // Reply result
                    let result = file::ReadResponse {
                        id: message.id,
                        result: Ok(data),
                    };
                    action.on_result.handle(world, result);
                }
                file::Action::Write(action) => {
                    event!(Level::DEBUG, "writing to file");

                    // TODO: Check write region is valid if offset is Some.

                    // Seek to given location
                    let seek_from = match action.offset {
                        Some(offset) => SeekFrom::Start(offset),
                        None => SeekFrom::End(0),
                    };
                    self.file.seek(seek_from)?;
                    let offset = self.file.stream_position()?;

                    // Perform the write
                    self.file.write_all(&action.data)?;

                    // Reply result
                    let result = file::WriteResponse {
                        id: message.id,
                        result: Ok(offset),
                    };
                    action.on_result.handle(world, result);
                }
            }
        }

        Ok(())
    }
}

impl Drop for SystemFile {
    fn drop(&mut self) {
        println!("DROPPING");
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
