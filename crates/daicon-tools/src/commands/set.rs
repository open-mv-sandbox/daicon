use anyhow::{Context, Error};
use clap::Args;
use daicon::{open_file_source, OpenMode, SourceAction, SourceMessage};
use ptero_file::{FileMessage, SystemFileServiceMessage};
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

#[instrument("set-command", skip_all)]
pub fn start(
    world: &mut World,
    system_file: Addr<SystemFileServiceMessage>,
    command: SetCommand,
) -> Result<(), Error> {
    event!(Level::INFO, "setting file in package");

    let id = world.create(None)?;
    let system = world.register(SetCommandSystem, id, SystemOptions::default());
    let addr = Addr::new(id);

    // Open the target file
    let message = SystemFileServiceMessage::Open {
        parent: Some(id),
        path: command.target.clone(),
        truncate: false,
        on_result: map_once(world, Some(id), addr, Message::FileOpened)?,
    };
    world.send(system_file, message);

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
                Message::FileOpened(file) => {
                    // Open up the package for writing in ptero-daicon
                    let source = open_file_source(world, Some(actor), file, OpenMode::ReadWrite)?;

                    // Add the data to the source
                    let instance = state.get(actor).context("failed to get instance")?;
                    let data = std::fs::read(&instance.input)?;
                    let message = SourceMessage {
                        id: Uuid::new_v4(),
                        action: SourceAction::Set {
                            id: instance.id,
                            data,
                            on_result: map_once(
                                world,
                                Some(actor),
                                Addr::new(actor),
                                Message::Write,
                            )?,
                        },
                    };
                    world.send(source, message);
                }
                Message::Write(()) => {
                    world.stop(actor)?;
                }
            }
        }

        Ok(())
    }
}

enum Message {
    FileOpened(Addr<FileMessage>),
    Write(()),
}
