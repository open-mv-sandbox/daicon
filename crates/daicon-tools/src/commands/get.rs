use anyhow::Error;
use clap::Args;
use daicon::{
    file::ReadResult,
    source::{open_file_source, OpenMode, SourceAction, SourceMessage},
};
use stewart::{Actor, ActorId, Addr, Options, State, World};
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
pub fn start(world: &mut World, command: GetCommand) -> Result<(), Error> {
    event!(Level::INFO, "getting file from package");

    let id = parse_hex(&command.id)?;

    let actor_id = world.create(None, Options::default())?;
    let addr = Addr::new(actor_id);

    // Open the target file
    let file = open_system_file(world, Some(actor_id), command.target.clone(), false)?;
    let source = open_file_source(world, Some(actor_id), file, OpenMode::ReadWrite)?;

    // Add the data to the source
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Get {
            id,
            on_result: map_once(world, Some(actor_id), addr, Message::Read)?,
        },
    };
    world.send(source, message);

    let actor = GetCommandActor {
        id: actor_id,
        command,
    };
    world.start(actor_id, actor)?;

    Ok(())
}

struct GetCommandActor {
    id: ActorId,
    command: GetCommand,
}

impl Actor for GetCommandActor {
    type Message = Message;

    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                Message::Read(result) => {
                    std::fs::write(&self.command.output, result.data)?;

                    // We're done
                    world.stop(self.id)?;
                }
            }
        }

        Ok(())
    }
}

enum Message {
    Read(ReadResult),
}
