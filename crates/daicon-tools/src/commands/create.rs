use anyhow::Error;
use clap::Args;
use daicon::{open_file_source, FileSourceOptions};
use daicon_native::open_system_file;
use stewart::{Actor, Context, Id, World};
use tracing::{event, instrument, Level};

/// Create a new daicon file.
#[derive(Args, Debug)]
pub struct Command {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,
}

#[instrument("daicon-tools::start_create", skip_all)]
pub fn start(world: &mut World, command: Command) -> Result<(), Error> {
    event!(Level::INFO, "creating package");

    let id = world.create(Id::none(), "command-create")?;

    // Open the target file
    let file = open_system_file(world, id, command.target.clone(), true)?;
    let _source = open_file_source(world, id, file, FileSourceOptions::default())?;

    // Start the command actor
    let actor = CreateCommandService {};
    world.start(id, actor)?;

    Ok(())
}

struct CreateCommandService {}

impl Actor for CreateCommandService {
    type Message = ();

    #[instrument("CreateCommandService", skip_all)]
    fn process(&mut self, _world: &mut World, _cx: Context<Self>) -> Result<(), Error> {
        Ok(())
    }
}
