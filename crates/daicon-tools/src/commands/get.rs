use anyhow::Error;
use clap::Args;
use daicon::{file::ReadResult, open_source, OpenMode, SourceAction, SourceGet, SourceMessage};
use daicon_native::open_system_file;
use stewart::{Actor, Context, Options, State};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::parse_hex;

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

#[instrument("GetCommandService", skip_all)]
pub fn start(ctx: &mut Context, command: GetCommand) -> Result<(), Error> {
    event!(Level::INFO, "getting file from package");

    let id = parse_hex(&command.id)?;

    let (mut ctx, sender) = ctx.create(Options::default())?;

    // Open the target file
    let file = open_system_file(&mut ctx, command.target.clone(), false)?;
    let source = open_source(&mut ctx, file, OpenMode::ReadWrite)?;

    // Add the data to the source
    let action = SourceGet {
        id,
        on_result: sender.map(Message::Read),
    };
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Get(action),
    };
    source.send(&mut ctx, message);

    let actor = GetCommandService { command };
    ctx.start(actor)?;

    Ok(())
}

struct GetCommandService {
    command: GetCommand,
}

impl Actor for GetCommandService {
    type Message = Message;

    #[instrument("GetCommandService", skip_all)]
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
