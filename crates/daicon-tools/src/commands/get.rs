use anyhow::{Context as _, Error};
use clap::Args;
use daicon::{open_file_source, OpenMode, SourceAction, SourceMessage};
use ptero_file::{FileMessage, ReadResult, SystemFileServiceMessage};
use stewart::{Addr, State, System, SystemOptions, World};
use stewart_utils::map_once;
use tracing::{event, instrument, Level};
use uuid::Uuid;

/// Get an entry from a daicon file.
#[derive(Args, Debug)]
pub struct GetCommand {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,

    /// UUID of the entry to get.
    #[arg(short = 'd', long, value_name = "UUID")]
    id: Uuid,

    /// Path of the output file to write.
    #[arg(short, long, value_name = "PATH")]
    output: String,
}

#[instrument("get-command", skip_all)]
pub fn start(
    world: &mut World,
    system_file: Addr<SystemFileServiceMessage>,
    command: GetCommand,
) -> Result<(), Error> {
    event!(Level::INFO, "getting file from package");

    let id = world.create(None)?;

    let system = world.register(GetCommandSystem, id, SystemOptions::default());
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

struct GetCommandSystem;

impl System for GetCommandSystem {
    type Instance = GetCommand;
    type Message = Message;

    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some((actor, message)) = state.next() {
            match message {
                Message::FileOpened(file) => {
                    // Open up the package for reading in ptero-daicon
                    let source = open_file_source(world, Some(actor), file, OpenMode::ReadWrite)?;

                    // Add the data to the source
                    let instance = state.get(actor).context("failed to get instance")?;
                    let message = SourceMessage {
                        id: Uuid::new_v4(),
                        action: SourceAction::Get {
                            id: instance.id,
                            on_result: map_once(
                                world,
                                Some(actor),
                                Addr::new(actor),
                                Message::Read,
                            )?,
                        },
                    };
                    world.send(source, message);
                }
                Message::Read(result) => {
                    let instance = state.get_mut(actor).context("failed to get instance")?;
                    std::fs::write(&instance.output, result.data)?;

                    // We're done
                    world.stop(actor)?;
                }
            }
        }

        Ok(())
    }
}

enum Message {
    FileOpened(Addr<FileMessage>),
    Read(ReadResult),
}
