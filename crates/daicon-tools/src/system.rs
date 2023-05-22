use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
};

use anyhow::{Context as _, Error};
use daicon::file::{FileAction, FileMessage, ReadResult, WriteLocation, WriteResult};
use stewart::{Actor, Context, Options, Sender, State};
use tracing::instrument;

#[instrument(skip_all)]
pub fn open_system_file(
    ctx: &mut Context,
    path: String,
    truncate: bool,
) -> Result<Sender<FileMessage>, Error> {
    let (mut ctx, sender) = ctx.create(Options::default())?;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(truncate)
        .create(true)
        .open(path)
        .context("failed to open system file for writing")?;

    let actor = SystemFile { file };
    ctx.start(actor)?;

    Ok(sender)
}

struct SystemFile {
    file: File,
}

impl Actor for SystemFile {
    type Message = FileMessage;

    #[instrument("system-file", skip_all)]
    fn process(&mut self, ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
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
                    on_result.send(ctx, result);
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
                    on_result.send(ctx, result);
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
