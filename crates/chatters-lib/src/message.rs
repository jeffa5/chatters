use std::{ops::Bound, path::PathBuf};

use crate::backends::{Contact, ContactId, Message, MessageContent, Quote};

#[derive(Debug)]
pub enum BackendMessage {
    LoadContacts,
    LoadMessages {
        contact_id: ContactId,
        start_ts: Bound<u64>,
        end_ts: Bound<u64>,
    },
    SendMessage {
        contact_id: ContactId,
        content: MessageContent,
        quote: Option<Quote>,
    },
    DownloadAttachment {
        contact_id: ContactId,
        timestamp: u64,
        index: usize,
    },
}

#[derive(Debug)]
pub enum FrontendMessage {
    LoadedContacts {
        contacts: Vec<Contact>,
    },
    LoadedMessages {
        messages: Vec<Message>,
    },
    NewMessage {
        message: Message,
    },
    DownloadedAttachment {
        contact_id: ContactId,
        timestamp: u64,
        index: usize,
        file_path: PathBuf,
    },
    Tick,
}
