use std::{cell::RefCell, collections::HashMap, ops::Range, rc::Rc};

use anyhow::{Context as _, Error};
use daicon::protocol::file;
use js_sys::{ArrayBuffer, Uint8Array};
use stewart::{Actor, Context, Handler, Id, World};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

#[instrument("open_fetch_file", skip_all)]
pub fn open_fetch_file(
    world: &mut World,
    id: Id,
    url: String,
    hnd: WorldHandle,
) -> Result<Handler<file::Request>, Error> {
    let id = world.create(id, "daicon-fetch-file")?;
    let handler = Handler::to(id);

    let actor = FetchFile {
        hnd,
        handler: handler.clone(),
        url,

        pending: HashMap::new(),
    };
    world.start(id, actor)?;

    Ok(handler.map(Message::Request))
}

struct FetchFile {
    hnd: WorldHandle,
    handler: Handler<Message>,
    url: String,

    pending: HashMap<Uuid, file::ReadAction>,
}

enum Message {
    Request(file::Request),
    FetchResult { id: Uuid, data: Vec<u8> },
}

impl Actor for FetchFile {
    type Message = Message;

    fn process(&mut self, world: &mut World, mut cx: Context<Self>) -> Result<(), Error> {
        while let Some(message) = cx.next() {
            match message {
                Message::Request(message) => {
                    self.on_message(world, message);
                }
                Message::FetchResult { id, data } => {
                    self.on_fetch_result(world, id, data)?;
                }
            }
        }

        Ok(())
    }
}

impl FetchFile {
    fn on_message(&mut self, world: &mut World, message: file::Request) {
        match message.action {
            file::Action::Read(action) => {
                self.on_read(message.id, action);
            }
            file::Action::Write(action) => {
                // Report back invalid operation
                let response = file::WriteResponse {
                    id: message.id,
                    result: Err(file::Error::NotSupported),
                };
                action.on_result.handle(world, response);
            }
        }
    }

    fn on_read(&mut self, id: Uuid, action: file::ReadAction) {
        event!(Level::INFO, "received read");

        let range = action.offset..(action.offset + action.size);
        self.pending.insert(id, action);

        // TODO: Batch fetches, we can do multiple range requests at once
        spawn_local(do_fetch(
            self.hnd.clone(),
            self.handler.clone(),
            id,
            self.url.clone(),
            range,
        ));
    }

    fn on_fetch_result(&mut self, world: &mut World, id: Uuid, data: Vec<u8>) -> Result<(), Error> {
        event!(Level::INFO, "received fetch result");

        // TODO: Validate the actual result, we may not have gotten what we asked for, for example
        //  a 200 response means we got the entire file instead of just the ranges.

        let pending = self.pending.remove(&id).context("failed to find pending")?;

        let message = file::ReadResponse {
            id,
            result: Ok(data),
        };
        pending.on_result.handle(world, message);

        Ok(())
    }
}

async fn do_fetch(
    hnd: WorldHandle,
    handler: Handler<Message>,
    id: Uuid,
    url: String,
    range: Range<u64>,
) {
    event!(Level::INFO, "fetching data");

    let window = web_sys::window().unwrap();

    // Perform fetch
    let headers = Headers::new().unwrap();
    let range_header = format!("bytes={}-{}", range.start, range.end - 1);
    event!(Level::TRACE, range = range_header);
    headers.append("Range", &range_header).unwrap();

    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);
    opts.headers(&headers);

    let request = Request::new_with_str_and_init(&url, &opts).unwrap();
    let response = window.fetch_with_request(&request);

    // Await the response
    let response = JsFuture::from(response).await.unwrap();
    let response: Response = response.dyn_into().unwrap();

    // Await all the response data
    let data = response.array_buffer().unwrap();
    let data = JsFuture::from(data).await.unwrap();
    let data: ArrayBuffer = data.dyn_into().unwrap();
    let data = Uint8Array::new(&data).to_vec();

    // Send the data back
    let mut world = hnd.borrow_mut();
    handler.handle(&mut world, Message::FetchResult { id, data });

    world.run_until_idle().unwrap();
}

/// TODO: Replace this with a more thought out executor abstraction.
pub type WorldHandle = Rc<RefCell<World>>;
