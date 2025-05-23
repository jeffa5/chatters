use futures::channel::mpsc;
use futures::channel::oneshot;
use std::future::Future;
use std::ops::Bound;
use std::path::Path;
use std::path::PathBuf;
use url::Url;

use crate::message::FrontendMessage;

#[derive(Debug, Clone, PartialEq)]
pub enum ContactId {
    User(Vec<u8>),
    Group(Vec<u8>),
}

impl std::fmt::Display for ContactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContactId::User(vec) => {
                write!(f, "User {}", hex::encode(vec))
            }
            ContactId::Group(vec) => {
                write!(f, "Group {}", hex::encode(vec))
            }
        }
    }
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
    Text {
        text: String,
        attachments: Vec<MessageAttachment>,
    },
    Reaction {
        message_author: Vec<u8>,
        timestamp: u64,
        reaction: String,
        remove: bool,
    },
    Edit {
        timestamp: u64,
        text: String,
    },
}

impl ToString for MessageContent {
    fn to_string(&self) -> String {
        match self {
            MessageContent::Text { text, .. } => text,
            MessageContent::Reaction { reaction, .. } => reaction,
            MessageContent::Edit { text, .. } => text,
        }
        .to_owned()
    }
}

#[derive(Debug, Clone)]
pub struct MessageAttachment {
    pub name: String,
    pub size: u64,
    pub index: usize,
    pub path: Option<PathBuf>,
}

impl MessageAttachment {
    pub fn file_name(&self) -> Option<String> {
        self.path
            .as_ref()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
    }

    pub fn human_size(&self) -> String {
        // starts in bytes
        let size = self.size;
        if size > 1_000_000_000 {
            format!("{}GB", size / 1_000_000_000)
        } else if size > 1_000_000 {
            format!("{}MB", size / 1_000_000)
        } else if size > 1_000 {
            format!("{}KB", size / 1_000)
        } else {
            format!("{}B", size)
        }
    }

    pub fn message_line(&self) -> String {
        let downloaded = self
            .file_name()
            .clone()
            .unwrap_or_else(|| "not downloaded".to_owned());
        format!("+ {} {} ({})", self.name, self.human_size(), downloaded)
    }
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
    pub last_message_timestamp: Option<u64>,
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

pub type Result<T> = std::result::Result<T, Error>;

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

    fn download_attachment(&self, attachment_index: usize)
        -> impl Future<Output = Result<PathBuf>>;
}

pub fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}
