use std::{cell::RefCell, rc::Rc};

use anyhow::Error;
use daicon::{
    open_file_source,
    protocol::{FileReadResponse, SourceAction, SourceGet, SourceMessage},
    OpenMode, OpenOptions,
};
use daicon_types::Id;
use daicon_web::open_fetch_file;
use stewart::{Actor, Context, Options, State, World};
use tracing::{event, Level};
use uuid::Uuid;

fn main() {
    tracing_wasm::set_as_global_default();

    event!(Level::INFO, "initializing world...");
    let world = World::new();
    let hnd = Rc::new(RefCell::new(world));

    let mut world = hnd.borrow_mut();
    let mut ctx = world.root();

    let (mut ctx, sender) = ctx.create(Options::default()).unwrap();

    event!(Level::INFO, "initializing fetch service...");
    let url = "http://localhost:8080/package.example";
    let file = open_fetch_file(&mut ctx, url.to_string(), hnd.clone()).unwrap();

    event!(Level::INFO, "initializing daicon service...");
    let source =
        open_file_source(&mut ctx, file, OpenMode::ReadWrite, OpenOptions::default()).unwrap();

    event!(Level::INFO, "starting example service...");
    ctx.start(ExampleService).unwrap();

    event!(Level::INFO, "dispatching requests...");
    let action = SourceGet {
        id: Id(0xbacc2ba1),
        on_result: sender.clone(),
    };
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Get(action),
    };
    source.send(&mut ctx, message);

    let action = SourceGet {
        id: Id(0x1f063ad4),
        on_result: sender,
    };
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Get(action),
    };
    source.send(&mut ctx, message);

    // Process everything
    world.run_until_idle().unwrap();
}

struct ExampleService;

impl Actor for ExampleService {
    type Message = FileReadResponse;

    fn process(&mut self, _ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            event!(Level::INFO, "received result");

            let text = std::str::from_utf8(&message.result)?;
            event!(Level::INFO, "text data:\n{}", text)
        }

        Ok(())
    }
}
