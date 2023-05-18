use anyhow::Error;
use clap::Args;
use daicon::{open_file_source, OpenMode, SourceAction, SourceMessage};
use ptero_file::open_system_file;
use stewart::{Addr, State, System, SystemOptions, World};
use stewart_utils::map_once;
use tracing::{event, instrument, Level};
use uuid::Uuid;

/// Set or add an entry in a daicon file.
#[derive(Args, Debug)]
pub struct SetCommand {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,

    /// UUID to assign the added data.
    #[arg(short = 'd', long, value_name = "UUID")]
    id: Uuid,

    /// Path of the input file to read.
    #[arg(short, long, value_name = "PATH")]
    input: String,
}

#[instrument("start_set_command", skip_all)]
pub fn start(world: &mut World, command: SetCommand) -> Result<(), Error> {
    event!(Level::INFO, "setting file in package");

    let id = world.create(None)?;
    let system = world.register(SetCommandSystem, id, SystemOptions::default());
    let addr = Addr::new(id);

    // Open the target file
    let file = open_system_file(world, Some(id), command.target.clone(), false)?;
    let source = open_file_source(world, Some(id), file, OpenMode::ReadWrite)?;

    // Add the data to the source
    let data = std::fs::read(&command.input)?;
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Set {
            id: command.id,
            data,
            on_result: map_once(world, Some(id), addr, Message::Write)?,
        },
    };
    world.send(source, message);

    world.start(id, system, command)?;

    Ok(())
}

struct SetCommandSystem;

impl System for SetCommandSystem {
    type Instance = SetCommand;
    type Message = Message;

    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some((actor, message)) = state.next() {
            match message {
                Message::Write(()) => {
                    world.stop(actor)?;
                }
            }
        }

        Ok(())
    }
}

enum Message {
    Write(()),
}
