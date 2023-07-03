use std::{cell::RefCell, collections::HashMap, ops::Range, rc::Rc};

use anyhow::{Context as _, Error};
use daicon::protocol::file;
use js_sys::{ArrayBuffer, Uint8Array};
use stewart::{Actor, Context, Handler, State, World};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

#[instrument("daicon-web::open_fetch_file", skip_all)]
pub fn open_fetch_file(
    world: &mut World,
    cx: &Context,
    url: String,
    hnd: WorldHandle,
) -> Result<Handler<file::Message>, Error> {
    let (_cx, id) = world.create(cx, "daicon-fetch-file")?;
    let handler = Handler::to(id);

    let actor = FetchFile {
        hnd,
        handler: handler.clone(),
        url,

        pending: HashMap::new(),
    };
    world.start(id, actor)?;

    Ok(handler.map(MessageImpl::Message))
}

struct FetchFile {
    hnd: WorldHandle,
    handler: Handler<MessageImpl>,
    url: String,

    pending: HashMap<Uuid, file::ReadAction>,
}

impl Actor for FetchFile {
    type Message = MessageImpl;

    fn process(
        &mut self,
        world: &mut World,
        _cx: &Context,
        state: &mut State<Self>,
    ) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                MessageImpl::Message(message) => {
                    self.on_message(world, message);
                }
                MessageImpl::FetchResult { id, data } => {
                    self.on_fetch_result(world, id, data)?;
                }
            }
        }

        Ok(())
    }
}

impl FetchFile {
    fn on_message(&mut self, world: &mut World, message: file::Message) {
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

enum MessageImpl {
    Message(file::Message),
    FetchResult { id: Uuid, data: Vec<u8> },
}

async fn do_fetch(
    hnd: WorldHandle,
    handler: Handler<MessageImpl>,
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
    handler.handle(&mut world, MessageImpl::FetchResult { id, data });

    let cx = Context::default();
    world.run_until_idle(&cx).unwrap();
}

/// TODO: Replace this with a more thought out executor abstraction.
pub type WorldHandle = Rc<RefCell<World>>;
