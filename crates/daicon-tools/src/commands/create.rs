use anyhow::Error;
use clap::Args;
use daicon::{open_file_source, OpenMode};
use ptero_file::open_system_file;
use stewart::{State, System, SystemOptions, World};
use tracing::{event, instrument, Level};

/// Create a new daicon file.
#[derive(Args, Debug)]
pub struct CreateCommand {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,
}

#[instrument("start_create_command", skip_all)]
pub fn start(world: &mut World, command: CreateCommand) -> Result<(), Error> {
    event!(Level::INFO, "creating package");

    let id = world.create(None)?;

    // Open the target file
    let file = open_system_file(world, Some(id), command.target.clone(), true)?;
    let _source = open_file_source(world, Some(id), file, OpenMode::Create)?;

    // Start the command actor
    let system = world.register(CreateCommandSystem, id, SystemOptions::default());
    world.start(id, system, command)?;

    Ok(())
}

struct CreateCommandSystem;

impl System for CreateCommandSystem {
    type Instance = CreateCommand;
    type Message = ();

    fn process(&mut self, _world: &mut World, _state: &mut State<Self>) -> Result<(), Error> {
        Ok(())
    }
}
