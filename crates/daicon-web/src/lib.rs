use std::{cell::RefCell, collections::HashMap, ops::Range, rc::Rc};

use anyhow::{Context as _, Error};
use daicon::protocol::file;
use js_sys::{ArrayBuffer, Uint8Array};
use stewart::{Actor, Context, Schedule, Sender, State, World};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

#[instrument("daicon-web::open_fetch_file", skip_all)]
pub fn open_fetch_file(
    ctx: &mut Context,
    url: String,
    hnd: WorldHandle,
) -> Result<Sender<file::Message>, Error> {
    let (mut ctx, sender) = ctx.create("fetch-file")?;

    let actor = FetchFile {
        hnd,
        sender: sender.clone(),
        url,

        pending: HashMap::new(),
    };
    ctx.start(actor)?;

    Ok(sender.map(MessageImpl::Message))
}

struct FetchFile {
    hnd: WorldHandle,
    sender: Sender<MessageImpl>,
    url: String,

    pending: HashMap<Uuid, file::ActionRead>,
}

impl Actor for FetchFile {
    type Message = MessageImpl;

    fn process(&mut self, ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                MessageImpl::Message(message) => {
                    self.on_message(ctx, message);
                }
                MessageImpl::FetchResult { id, data } => {
                    self.on_fetch_result(ctx, id, data)?;
                }
            }
        }

        Ok(())
    }
}

impl FetchFile {
    fn on_message(&mut self, ctx: &mut Context, message: file::Message) {
        match message.action {
            file::Action::Read(action) => {
                self.on_read(message.id, action);
            }
            file::Action::Write(action) => {
                // Report back invalid operation
                let response = file::ActionWriteResponse {
                    id: message.id,
                    result: Err(file::Error::ActionNotSupported),
                };
                action.on_result.send(ctx, response);
            }
            file::Action::Append(action) => {
                // Report back invalid operation
                let response = file::ActionAppendResponse {
                    id: message.id,
                    result: Err(file::Error::ActionNotSupported),
                };
                action.on_result.send(ctx, response);
            }
        }
    }

    fn on_read(&mut self, id: Uuid, action: file::ActionRead) {
        event!(Level::INFO, "received read");

        let range = action.offset..(action.offset + action.size);
        self.pending.insert(id, action);

        // TODO: Batch fetches, we can do multiple range requests at once
        spawn_local(do_fetch(
            self.hnd.clone(),
            self.sender.clone(),
            id,
            self.url.clone(),
            range,
        ));
    }

    fn on_fetch_result(&mut self, ctx: &mut Context, id: Uuid, data: Vec<u8>) -> Result<(), Error> {
        event!(Level::INFO, "received fetch result");

        // TODO: Validate the actual result, we may not have gotten what we asked for, for example
        //  a 200 response means we got the entire file instead of just the ranges.

        let pending = self.pending.remove(&id).context("failed to find pending")?;

        let message = file::ActionReadResponse {
            id,
            result: Ok(data),
        };
        pending.on_result.send(ctx, message);

        Ok(())
    }
}

enum MessageImpl {
    Message(file::Message),
    FetchResult { id: Uuid, data: Vec<u8> },
}

async fn do_fetch(
    hnd: WorldHandle,
    sender: Sender<MessageImpl>,
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
    let mut schedule = Schedule::default();
    let mut ctx = Context::root(&mut world, &mut schedule);

    sender.send(&mut ctx, MessageImpl::FetchResult { id, data });
    schedule.run_until_idle(&mut world).unwrap();
}

/// TODO: Replace this with a more thought out executor abstraction.
pub type WorldHandle = Rc<RefCell<World>>;
