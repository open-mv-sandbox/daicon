use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::{Context as _, Error};
use daicon::protocol::{FileAction, FileMessage, FileRead, ReadResult};
use js_sys::{ArrayBuffer, Uint8Array};
use stewart::{Actor, Context, Options, Sender, State, World};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Request, RequestInit, RequestMode, Response};

#[instrument("SystemFile", skip_all)]
pub fn open_fetch_file(
    ctx: &mut Context,
    url: String,
    hnd: WorldHandle,
) -> Result<Sender<FileMessage>, Error> {
    let (mut ctx, sender) = ctx.create(Options::default())?;

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

    pending: HashMap<Uuid, FileRead>,
}

impl Actor for FetchFile {
    type Message = MessageImpl;

    #[instrument("SystemFile", skip_all)]
    fn process(&mut self, ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                MessageImpl::Message(message) => {
                    self.on_message(message);
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
    fn on_message(&mut self, message: FileMessage) {
        match message.action {
            FileAction::Read(action) => {
                self.on_read(message.id, action);
            }
            FileAction::Write { .. } => {
                // TODO: Report back invalid operation
            }
        }
    }

    fn on_read(&mut self, id: Uuid, action: FileRead) {
        event!(Level::INFO, "received read");

        // TODO: VERY IMPORTANT: Batch all fetches, and only fetch correct regions
        self.pending.insert(id, action);
        spawn_local(do_fetch(
            self.hnd.clone(),
            self.sender.clone(),
            id,
            self.url.clone(),
        ));
    }

    fn on_fetch_result(&mut self, ctx: &mut Context, id: Uuid, data: Vec<u8>) -> Result<(), Error> {
        event!(Level::INFO, "received fetch result");

        let pending = self.pending.remove(&id).context("failed to find pending")?;

        let offset = pending.offset as usize;
        let mut reply_data = vec![0u8; pending.size as usize];

        let available = data.len() - offset;
        let slice_len = usize::min(reply_data.len(), available);

        let src = &data[offset..offset + slice_len];
        let dst = &mut reply_data[0..slice_len];

        dst.copy_from_slice(src);

        let message = ReadResult {
            id,
            offset: pending.offset,
            data: reply_data,
        };
        pending.on_result.send(ctx, message);

        Ok(())
    }
}

enum MessageImpl {
    Message(FileMessage),
    FetchResult { id: Uuid, data: Vec<u8> },
}

async fn do_fetch(hnd: WorldHandle, sender: Sender<MessageImpl>, id: Uuid, url: String) {
    event!(Level::INFO, "fetching data");

    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts).unwrap();

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .unwrap();

    let resp: Response = resp_value.dyn_into().unwrap();
    let data = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
    let data: ArrayBuffer = data.dyn_into().unwrap();
    let data = Uint8Array::new(&data).to_vec();

    // Send the data back
    let mut world = hnd.borrow_mut();
    let mut ctx = world.root();
    sender.send(&mut ctx, MessageImpl::FetchResult { id, data });
    world.run_until_idle().unwrap();
}

/// TODO: Replace this with a more thought out executor abstraction.
pub type WorldHandle = Rc<RefCell<World>>;
