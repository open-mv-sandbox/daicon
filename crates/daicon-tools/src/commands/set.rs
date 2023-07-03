use anyhow::Error;
use clap::Args;
use daicon::{open_file_source, protocol::source, FileSourceOptions};
use daicon_native::open_system_file;
use stewart::{Actor, Context, Handler, State, World};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::parse_hex;

/// Set or add an entry in a daicon file.
#[derive(Args, Debug)]
pub struct Command {
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

#[instrument("daicon-tools::start_set", skip_all)]
pub fn start(world: &mut World, cx: &Context, command: Command) -> Result<(), Error> {
    event!(Level::INFO, "setting file in package");

    let asset_id = parse_hex(&command.id)?;

    let (cx, id) = world.create(cx, "command-set")?;
    let handler = Handler::to(id);

    // Open the target file
    let file = open_system_file(world, &cx, command.target.clone(), false)?;
    let options = FileSourceOptions::default().open_table(0);
    let source = open_file_source(world, &cx, file, options)?;

    // Add the data to the source
    let data = std::fs::read(&command.input)?;
    let action = source::SetAction {
        id: asset_id,
        data,
        on_result: handler.map(Message::Result),
    };
    let message = source::Message {
        id: Uuid::new_v4(),
        action: source::Action::Set(action),
    };
    source.handle(world, message);

    let actor = SetCommandService {};
    world.start(id, actor)?;

    Ok(())
}

struct SetCommandService {}

impl Actor for SetCommandService {
    type Message = Message;

    #[instrument("SetCommandService", skip_all)]
    fn process(
        &mut self,
        _world: &mut World,
        _cx: &Context,
        state: &mut State<Self>,
    ) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                Message::Result(_) => {
                    state.stop();
                }
            }
        }

        Ok(())
    }
}

enum Message {
    Result(source::SetResponse),
}
