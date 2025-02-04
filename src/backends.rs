use futures::channel::mpsc;
use futures::channel::oneshot;
use std::future::Future;
use std::ops::Bound;
use std::path::Path;
use std::path::PathBuf;
use url::Url;

use crate::message::FrontendMessage;

pub mod local;
pub mod matrix;
pub mod signal;

#[derive(Debug, Clone, PartialEq)]
pub enum ContactId {
    User(Vec<u8>),
    Group(Vec<u8>),
}

#[derive(Debug)]
pub struct Message {
    pub timestamp: u64,
    pub sender: Vec<u8>,
    pub contact_id: ContactId,
    pub content: MessageContent,
    pub quote: Option<Quote>,
}

#[derive(Debug)]
pub enum MessageContent {
    Text(String, Vec<MessageAttachment>),
    Reaction(Vec<u8>, u64, String, bool),
}

impl ToString for MessageContent {
    fn to_string(&self) -> String {
        match self {
            MessageContent::Text(t, _) => t,
            MessageContent::Reaction(_, _, r, _) => r,
        }
        .to_owned()
    }
}

#[derive(Debug)]
pub struct MessageAttachment {
    pub name: String,
    pub size: u32,
    pub index: usize,
    pub downloaded_name: Option<String>,
    pub downloaded_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct Quote {
    pub timestamp: u64,
    pub sender: Vec<u8>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Contact {
    pub id: ContactId,
    pub name: String,
    pub address: String,
    pub last_message_timestamp: u64,
    pub description: String,
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

    fn background_sync(
        &mut self,
        ba_tx: mpsc::UnboundedSender<FrontendMessage>,
    ) -> impl Future<Output = Result<()>>;

    fn users(&self) -> impl Future<Output = Result<Vec<Contact>>>;

    fn groups(&self) -> impl Future<Output = Result<Vec<Contact>>>;

    fn messages(
        &mut self,
        contact_id: ContactId,
        start_ts: Bound<u64>,
        end_ts: Bound<u64>,
    ) -> impl Future<Output = Result<Vec<Message>>>;

    fn send_message(
        &mut self,
        contact_id: ContactId,
        body: MessageContent,
        quoting: Option<&Quote>,
    ) -> impl Future<Output = Result<Message>>;

    fn self_id(&self) -> impl Future<Output = Vec<u8>>;

    fn download_attachment(&self, attachment_index: usize) -> impl Future<Output = Result<String>>;
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}
