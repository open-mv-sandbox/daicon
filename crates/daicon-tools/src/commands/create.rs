use anyhow::Error;
use clap::Args;
use daicon::{open_file_source, FileSourceOptions};
use daicon_native::open_system_file;
use stewart::{Actor, Context, State};
use tracing::{event, instrument, Level};

/// Create a new daicon file.
#[derive(Args, Debug)]
pub struct CreateCommand {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,
}

#[instrument("daicon-tools::start_create", skip_all)]
pub fn start(ctx: &mut Context, command: CreateCommand) -> Result<(), Error> {
    event!(Level::INFO, "creating package");

    let (mut ctx, _) = ctx.create::<()>("command-create")?;

    // Open the target file
    let file = open_system_file(&mut ctx, command.target.clone(), true)?;
    let _source = open_file_source(&mut ctx, file, FileSourceOptions::default())?;

    // Start the command actor
    let actor = CreateCommandService {};
    ctx.start(actor)?;

    Ok(())
}

struct CreateCommandService {}

impl Actor for CreateCommandService {
    type Message = ();

    #[instrument("CreateCommandService", skip_all)]
    fn process(&mut self, _ctx: &mut Context, _state: &mut State<Self>) -> Result<(), Error> {
        Ok(())
    }
}
