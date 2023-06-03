use anyhow::Error;
use clap::Args;
use daicon::{open_file_source, protocol::source, FileSourceOptions};
use daicon_native::open_system_file;
use stewart::{Actor, Context, State};
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

#[instrument("daicon-tools::start_get", skip_all)]
pub fn start(ctx: &mut Context, command: GetCommand) -> Result<(), Error> {
    event!(Level::INFO, "getting file from package");

    let id = parse_hex(&command.id)?;

    let (mut ctx, sender) = ctx.create("command-get")?;

    // Open the target file
    let file = open_system_file(&mut ctx, command.target.clone(), false)?;
    let options = FileSourceOptions::default().first_table(0);
    let source = open_file_source(&mut ctx, file, options)?;

    // Add the data to the source
    let action = source::GetAction {
        id,
        on_result: sender.map(Message::Result),
    };
    let message = source::Message {
        id: Uuid::new_v4(),
        action: source::Action::Get(action),
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

    fn process(&mut self, _ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                Message::Result(response) => {
                    let data = response.result?;
                    std::fs::write(&self.command.output, data)?;

                    // We're done
                    state.stop();
                }
            }
        }

        Ok(())
    }
}

enum Message {
    Result(source::GetResponse),
}
