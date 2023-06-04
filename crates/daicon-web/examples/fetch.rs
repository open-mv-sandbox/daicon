use std::{cell::RefCell, rc::Rc};

use anyhow::Error;
use daicon::{
    open_file_source,
    protocol::source::{self, Id},
    FileSourceOptions,
};
use daicon_web::open_fetch_file;
use stewart::{Actor, Context, Schedule, State, World};
use tracing::{event, Level};
use uuid::Uuid;

fn main() {
    tracing_wasm::set_as_global_default();

    event!(Level::INFO, "initializing world...");
    let world = World::default();
    let mut schedule = Schedule::default();
    let hnd = Rc::new(RefCell::new(world));

    let mut world = hnd.borrow_mut();
    let mut ctx = Context::root(&mut world, &mut schedule);

    let (mut ctx, sender) = ctx.create("fetch-example").unwrap();

    event!(Level::INFO, "initializing fetch service...");
    let url = "http://localhost:8080/package.example";
    let file = open_fetch_file(&mut ctx, url.to_string(), hnd.clone()).unwrap();

    event!(Level::INFO, "initializing daicon service...");
    let options = FileSourceOptions::default().open_table(0);
    let source = open_file_source(&mut ctx, file, options).unwrap();

    event!(Level::INFO, "starting example service...");
    ctx.start(ExampleService).unwrap();

    event!(Level::INFO, "dispatching requests...");
    let action = source::GetAction {
        id: Id(0xbacc2ba1),
        on_result: sender.clone(),
    };
    let message = source::Message {
        id: Uuid::new_v4(),
        action: source::Action::Get(action),
    };
    source.send(&mut ctx, message);

    let action = source::GetAction {
        id: Id(0x1f063ad4),
        on_result: sender,
    };
    let message = source::Message {
        id: Uuid::new_v4(),
        action: source::Action::Get(action),
    };
    source.send(&mut ctx, message);

    // Process everything
    schedule.run_until_idle(&mut world).unwrap();
}

struct ExampleService;

impl Actor for ExampleService {
    type Message = source::GetResponse;

    fn process(&mut self, _ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            event!(Level::INFO, "received result");

            let data = message.result?;
            let text = std::str::from_utf8(&data)?;
            event!(Level::INFO, "text data:\n{}", text)
        }

        Ok(())
    }
}
