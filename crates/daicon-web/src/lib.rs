use std::{cell::RefCell, rc::Rc};

use anyhow::Error;
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
    hnd: SystemH,
) -> Result<Sender<FileMessage>, Error> {
    let (mut ctx, sender) = ctx.create(Options::default())?;

    let actor = FetchFile {
        data: None,
        pending: Vec::new(),
    };
    ctx.start(actor)?;

    // TODO: Do fetch requests on-demand for data rather than fetching everything at once
    spawn_local(do_fetch(hnd, sender.clone(), url));

    Ok(sender.map(MessageImpl::Message))
}

struct FetchFile {
    data: Option<Vec<u8>>,
    pending: Vec<Pending>,
}

struct Pending {
    id: Uuid,
    action: FileRead,
}

impl Actor for FetchFile {
    type Message = MessageImpl;

    #[instrument("SystemFile", skip_all)]
    fn process(&mut self, ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                MessageImpl::Message(message) => {
                    match message.action {
                        FileAction::Read(action) => {
                            let pending = Pending {
                                id: message.id,
                                action,
                            };
                            self.pending.push(pending);
                        }
                        FileAction::Write { .. } => {
                            // TODO: Report back invalid operation
                        }
                    }
                }
                MessageImpl::FetchResult(data) => {
                    event!(Level::INFO, "fetch response received");
                    self.data = Some(data);
                }
            }
        }

        // Check if we can respond to accumulated requests
        if let Some(data) = &self.data {
            event!(Level::INFO, count = self.pending.len(), "resolving entries");

            for pending in self.pending.drain(..) {
                let offset = pending.action.offset as usize;
                let mut reply_data = vec![0u8; pending.action.size as usize];

                let available = data.len() - offset;
                let slice_len = usize::min(reply_data.len(), available);

                let src = &data[offset..offset + slice_len];
                let dst = &mut reply_data[0..slice_len];

                dst.copy_from_slice(src);

                let message = ReadResult {
                    id: pending.id,
                    offset: pending.action.offset,
                    data: reply_data,
                };
                pending.action.on_result.send(ctx, message);
            }
        }

        Ok(())
    }
}

enum MessageImpl {
    Message(FileMessage),
    FetchResult(Vec<u8>),
}

async fn do_fetch(hnd: SystemH, sender: Sender<MessageImpl>, url: String) {
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
    sender.send(&mut ctx, MessageImpl::FetchResult(data));
    world.run_until_idle().unwrap();
}

/// TODO: Replace this with a more thought out executor abstraction.
pub type SystemH = Rc<RefCell<World>>;
