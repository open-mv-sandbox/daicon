use anyhow::Error;
use clap::Args;
use daicon::{open_source, OpenMode, SourceAction, SourceMessage};
use stewart::{Actor, ActorId, Addr, Options, State, World};
use stewart_utils::map_once;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{parse_hex, system::open_system_file};

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

#[instrument("start_set_command", skip_all)]
pub fn start(world: &mut World, command: SetCommand) -> Result<(), Error> {
    event!(Level::INFO, "setting file in package");

    let id = parse_hex(&command.id)?;

    let actor_id = world.create(None, Options::default())?;
    let addr = Addr::new(actor_id);

    // Open the target file
    let file = open_system_file(world, Some(actor_id), command.target.clone(), false)?;
    let source = open_source(world, Some(actor_id), file, OpenMode::ReadWrite)?;

    // Add the data to the source
    let data = std::fs::read(&command.input)?;
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Set {
            id,
            data,
            on_result: map_once(world, Some(actor_id), addr, Message::Write)?,
        },
    };
    world.send(source, message);

    let actor = SetCommandActor { id: actor_id };
    world.start(actor_id, actor)?;

    Ok(())
}

struct SetCommandActor {
    id: ActorId,
}

impl Actor for SetCommandActor {
    type Message = Message;

    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                Message::Write(()) => {
                    world.stop(self.id)?;
                }
            }
        }

        Ok(())
    }
}

enum Message {
    Write(()),
}
