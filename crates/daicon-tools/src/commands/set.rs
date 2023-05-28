use anyhow::Error;
use clap::Args;
use daicon::{
    open_file_source,
    protocol::{SourceAction, SourceMessage, SourceSet},
    OpenMode, OpenOptions,
};
use daicon_native::open_system_file;
use stewart::{Actor, Context, State};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::parse_hex;

/// Set or add an entry in a daicon file.
#[derive(Args, Debug)]
pub struct SetCommand {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,

    /// Id in hexadecimal to assign the added data.
    #[arg(short = 'd', long, value_name = "ID")]
    id: String,

    /// Path of the input file to read.
    #[arg(short, long, value_name = "PATH")]
    input: String,
}

#[instrument("SetCommandService", skip_all)]
pub fn start(ctx: &mut Context, command: SetCommand) -> Result<(), Error> {
    event!(Level::INFO, "setting file in package");

    let id = parse_hex(&command.id)?;

    let (mut ctx, sender) = ctx.create()?;

    // Open the target file
    let file = open_system_file(&mut ctx, command.target.clone(), false)?;
    let source = open_file_source(&mut ctx, file, OpenMode::ReadWrite, OpenOptions::default())?;

    // Add the data to the source
    let data = std::fs::read(&command.input)?;
    let action = SourceSet {
        id,
        data,
        on_result: sender.map(Message::Write),
    };
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Set(action),
    };
    source.send(&mut ctx, message);

    let actor = SetCommandService {};
    ctx.start(actor)?;

    Ok(())
}

struct SetCommandService {}

impl Actor for SetCommandService {
    type Message = Message;

    #[instrument("SetCommandService", skip_all)]
    fn process(&mut self, ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                Message::Write(()) => {
                    ctx.stop()?;
                }
            }
        }

        Ok(())
    }
}

enum Message {
    Write(()),
}
