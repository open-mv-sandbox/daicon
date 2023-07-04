use anyhow::Error;
use clap::Args;
use daicon::{open_file_source, protocol::source, FileSourceOptions};
use daicon_native::open_system_file;
use stewart::{Actor, Context, Handler, Id, World};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::parse_hex;

/// Get an entry from a daicon file.
#[derive(Args, Debug)]
pub struct Command {
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
pub fn start(world: &mut World, command: Command) -> Result<(), Error> {
    event!(Level::INFO, "getting file from package");

    let asset_id = parse_hex(&command.id)?;

    let id = world.create(Id::none(), "command-get")?;
    let handler = Handler::to(id);

    // Open the target file
    let file = open_system_file(world, id, command.target.clone(), false)?;
    let options = FileSourceOptions::default().open_table(0);
    let source = open_file_source(world, id, file, options)?;

    // Add the data to the source
    let action = source::GetAction {
        id: asset_id,
        on_result: handler.map(Message::Result),
    };
    let message = source::Message {
        id: Uuid::new_v4(),
        action: source::Action::Get(action),
    };
    source.handle(world, message);

    let actor = GetCommandService { command };
    world.start(id, actor)?;

    Ok(())
}

struct GetCommandService {
    command: Command,
}

impl Actor for GetCommandService {
    type Message = Message;

    fn process(&mut self, _world: &mut World, mut cx: Context<Self>) -> Result<(), Error> {
        while let Some(message) = cx.next() {
            match message {
                Message::Result(response) => {
                    let data = response.result?;
                    std::fs::write(&self.command.output, data)?;

                    // We're done
                    cx.stop();
                }
            }
        }

        Ok(())
    }
}

enum Message {
    Result(source::GetResponse),
}
