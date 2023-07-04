use std::{cell::RefCell, rc::Rc};

use anyhow::{Context as _, Error};
use daicon::{open_file_source, protocol::source, FileSourceOptions};
use daicon_web::open_fetch_file;
use stewart::{Actor, Context, Handler, Id, World};
use tracing::{event, Level};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

fn main() {
    tracing_wasm::set_as_global_default();

    event!(Level::INFO, "initializing world...");
    let world = World::default();
    let hnd = Rc::new(RefCell::new(world));

    let mut world = hnd.borrow_mut();

    let id = world.create(Id::none(), "fetch-example").unwrap();
    let handler = Handler::to(id);

    event!(Level::INFO, "initializing fetch service...");
    let url = "http://localhost:8080/package.example";
    let file = open_fetch_file(&mut world, id, url.to_string(), hnd.clone()).unwrap();

    event!(Level::INFO, "initializing daicon service...");
    let options = FileSourceOptions::default().open_table(0);
    let source = open_file_source(&mut world, id, file, options).unwrap();

    event!(Level::INFO, "starting example service...");
    world.start(id, ExampleService).unwrap();

    event!(Level::INFO, "dispatching requests...");
    let action = source::GetAction {
        id: source::Id(0xbacc2ba1),
        on_result: handler.clone(),
    };
    let message = source::Request {
        id: Uuid::new_v4(),
        action: source::Action::Get(action),
    };
    source.handle(&mut world, message);

    let action = source::GetAction {
        id: source::Id(0x1f063ad4),
        on_result: handler,
    };
    let message = source::Request {
        id: Uuid::new_v4(),
        action: source::Action::Get(action),
    };
    source.handle(&mut world, message);

    // Process everything
    world.run_until_idle().unwrap();
}

struct ExampleService;

impl Actor for ExampleService {
    type Message = source::GetResponse;

    fn process(&mut self, _world: &mut World, mut cx: Context<Self>) -> Result<(), Error> {
        while let Some(message) = cx.next() {
            event!(Level::INFO, "received result");

            // Decode the data as a text file
            let data = message.result?;
            let text = std::str::from_utf8(&data)?;

            // Log what we've received
            event!(Level::INFO, "text data:\n{}", text);

            // Add it to the list
            let document = web_sys::window()
                .context("failed to get window")?
                .document()
                .context("failed to get document")?;

            let list = document
                .get_element_by_id("output")
                .context("failed to find output")?;
            let list: HtmlElement = list.dyn_into().ok().context("incorrect element")?;

            let text = format!("==== Get Request {} ====\n\n{}\n\n", message.id, text);
            let node = document
                .create_element("pre")
                .ok()
                .context("failed to create node")?;
            node.set_text_content(Some(&text));
            list.append_child(&node).ok().context("failed to append")?;
        }

        Ok(())
    }
}
