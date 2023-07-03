use std::{cell::RefCell, rc::Rc};

use anyhow::Error;
use daicon::{
    open_file_source,
    protocol::source::{self, Id},
    FileSourceOptions,
};
use daicon_web::open_fetch_file;
use stewart::{Actor, Context, Handler, State, World};
use tracing::{event, Level};
use uuid::Uuid;

fn main() {
    tracing_wasm::set_as_global_default();

    event!(Level::INFO, "initializing world...");
    let world = World::default();
    let hnd = Rc::new(RefCell::new(world));

    let mut world = hnd.borrow_mut();
    let cx = Context::default();

    let (cx, id) = world.create(&cx, "fetch-example").unwrap();
    let handler = Handler::to(id);

    event!(Level::INFO, "initializing fetch service...");
    let url = "http://localhost:8080/package.example";
    let file = open_fetch_file(&mut world, &cx, url.to_string(), hnd.clone()).unwrap();

    event!(Level::INFO, "initializing daicon service...");
    let options = FileSourceOptions::default().open_table(0);
    let source = open_file_source(&mut world, &cx, file, options).unwrap();

    event!(Level::INFO, "starting example service...");
    world.start(id, ExampleService).unwrap();

    event!(Level::INFO, "dispatching requests...");
    let action = source::GetAction {
        id: Id(0xbacc2ba1),
        on_result: handler.clone(),
    };
    let message = source::Message {
        id: Uuid::new_v4(),
        action: source::Action::Get(action),
    };
    source.handle(&mut world, message);

    let action = source::GetAction {
        id: Id(0x1f063ad4),
        on_result: handler,
    };
    let message = source::Message {
        id: Uuid::new_v4(),
        action: source::Action::Get(action),
    };
    source.handle(&mut world, message);

    // Process everything
    world.run_until_idle(&cx).unwrap();
}

struct ExampleService;

impl Actor for ExampleService {
    type Message = source::GetResponse;

    fn process(
        &mut self,
        _world: &mut World,
        _cx: &Context,
        state: &mut State<Self>,
    ) -> Result<(), Error> {
        while let Some(message) = state.next() {
            event!(Level::INFO, "received result");

            let data = message.result?;
            let text = std::str::from_utf8(&data)?;
            event!(Level::INFO, "text data:\n{}", text)
        }

        Ok(())
    }
}
