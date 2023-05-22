use anyhow::Error;
use clap::Args;
use daicon::{open_source, OpenMode};
use stewart::{Actor, Context, Options, State};
use tracing::{event, instrument, Level};

use crate::system::open_system_file;

/// Create a new daicon file.
#[derive(Args, Debug)]
pub struct CreateCommand {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,
}

#[instrument("start_create_command", skip_all)]
pub fn start(ctx: &mut Context, command: CreateCommand) -> Result<(), Error> {
    event!(Level::INFO, "creating package");

    let mut ctx = ctx.create(Options::default())?;

    // Open the target file
    let file = open_system_file(&mut ctx, command.target.clone(), true)?;
    let _source = open_source(&mut ctx, file, OpenMode::Create)?;

    // Start the command actor
    ctx.start(command)?;

    Ok(())
}

impl Actor for CreateCommand {
    type Message = ();

    fn process(&mut self, _ctx: &mut Context, _state: &mut State<Self>) -> Result<(), Error> {
        Ok(())
    }
}
