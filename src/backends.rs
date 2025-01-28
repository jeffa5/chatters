use futures::channel::mpsc;
use futures::channel::oneshot;
use presage::libsignal_service::prelude::Uuid;
use presage::store::Thread;
use std::future::Future;
use std::ops::Bound;
use std::path::Path;
use std::path::PathBuf;
use url::Url;

use crate::message::FrontendMessage;

pub mod local;
pub mod signal;

#[derive(Debug)]
pub struct Message {
    pub timestamp: u64,
    pub sender: Uuid,
    pub thread: Thread,
    pub content: MessageContent,
}

#[derive(Debug)]
pub enum MessageContent {
    Text(String, Vec<MessageAttachment>),
    Reaction(Uuid, u64, String, bool),
}

#[derive(Debug)]
pub struct MessageAttachment {
    pub name: String,
    pub size: u32,
    pub index: usize,
    pub downloaded_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Contact {
    pub thread_id: Thread,
    pub name: String,
    pub address: String,
    pub last_message_timestamp: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unlinked")]
    Unlinked,
    #[error("Unknown attachment with index {0}")]
    UnknownAttachment(usize),
    #[error("A failure occurred: {0}")]
    Failure(String),
}

type Result<T> = std::result::Result<T, Error>;

pub trait Backend: Sized {
    fn load(path: &Path) -> impl Future<Output = Result<Self>>;

    fn link(
        path: &Path,
        device_name: &str,
        provisioning_link_tx: oneshot::Sender<Url>,
    ) -> impl Future<Output = Result<Self>>;

    fn sync_contacts(&mut self) -> impl Future<Output = Result<()>>;

    fn background_sync(
        &mut self,
        ba_tx: mpsc::UnboundedSender<FrontendMessage>,
    ) -> impl Future<Output = Result<()>>;

    fn contacts(&self) -> impl Future<Output = Result<Vec<Contact>>>;

    fn groups(&self) -> impl Future<Output = Result<Vec<Contact>>>;

    fn messages(
        &mut self,
        contact: Thread,
        start_ts: Bound<u64>,
        end_ts: Bound<u64>,
    ) -> impl Future<Output = Result<Vec<Message>>>;

    fn send_message(
        &mut self,
        contact: Thread,
        body: MessageContent,
    ) -> impl Future<Output = Result<Message>>;

    fn self_uuid(&self) -> impl Future<Output = Uuid>;

    fn download_attachment(&self, attachment_index: usize)
        -> impl Future<Output = Result<PathBuf>>;
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}
