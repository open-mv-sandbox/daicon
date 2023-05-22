use anyhow::Error;
use clap::Args;
use daicon::{file::ReadResult, open_source, OpenMode, SourceAction, SourceMessage};
use stewart::{Actor, Context, Options, State};
use stewart_utils::map_once;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{parse_hex, system::open_system_file};

/// Get an entry from a daicon file.
#[derive(Args, Debug)]
pub struct GetCommand {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,

    /// Id in hexadecimal of the entry to get.
    #[arg(short = 'd', long, value_name = "ID")]
    id: String,

    /// Path of the output file to write.
    #[arg(short, long, value_name = "PATH")]
    output: String,
}

#[instrument("start_get_command", skip_all)]
pub fn start(ctx: &mut Context, command: GetCommand) -> Result<(), Error> {
    event!(Level::INFO, "getting file from package");

    let id = parse_hex(&command.id)?;

    let mut ctx = ctx.create(Options::default())?;
    let addr = ctx.addr()?;

    // Open the target file
    let file = open_system_file(&mut ctx, command.target.clone(), false)?;
    let source = open_source(&mut ctx, file, OpenMode::ReadWrite)?;

    // Add the data to the source
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Get {
            id,
            on_result: map_once(&mut ctx, addr, Message::Read)?,
        },
    };
    ctx.send(source, message);

    let actor = GetCommandActor { command };
    ctx.start(actor)?;

    Ok(())
}

struct GetCommandActor {
    command: GetCommand,
}

impl Actor for GetCommandActor {
    type Message = Message;

    fn process(&mut self, ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                Message::Read(result) => {
                    std::fs::write(&self.command.output, result.data)?;

                    // We're done
                    ctx.stop()?;
                }
            }
        }

        Ok(())
    }
}

enum Message {
    Read(ReadResult),
}
