mod commands;

use anyhow::{bail, Error};
use clap::{Parser, Subcommand};
use daicon_types::Id;
use stewart::{Context, World};
use tracing::{event, Level};
use tracing_subscriber::{prelude::*, EnvFilter, FmtSubscriber};

use crate::commands::{create, get, set};

fn main() {
    let args = CliArgs::parse();

    let filter = EnvFilter::builder().parse("trace,stewart=info").unwrap();
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .without_time()
        .with_target(false)
        .finish()
        .with(filter);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // Run main
    let result = try_main(args);

    // Report any otherwise unhandled errors
    if let Err(error) = result {
        event!(Level::ERROR, "failed:\n{:?}", error);
        std::process::exit(1);
    }
}

fn try_main(args: CliArgs) -> Result<(), Error> {
    // Set up the runtime
    let mut world = World::default();
    let cx = Context::default();

    // Start the command actor
    match args.command {
        Command::Create(command) => commands::create::start(&mut world, &cx, command)?,
        Command::Set(command) => commands::set::start(&mut world, &cx, command)?,
        Command::Get(command) => commands::get::start(&mut world, &cx, command)?,
    };

    // Run the command until it's done
    world.run_until_idle(&cx)?;

    // TODO: Receive command errors
    Ok(())
}

/// Pterodactil CLI toolkit for working with dacti packages.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Create(create::Command),
    Set(set::Command),
    Get(get::Command),
}

fn parse_hex(str: &str) -> Result<Id, Error> {
    if str.len() != 10 || !str.starts_with("0x") {
        bail!("input must be a hexadecimal, starting with 0x, followed by 8 characters");
    }

    let result = u32::from_str_radix(&str[2..], 16)?;

    Ok(Id(result))
}
