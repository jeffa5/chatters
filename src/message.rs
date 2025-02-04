use std::ops::Bound;

use crate::backends::{Contact, ContactId, Message, MessageContent, Quote};

#[derive(Debug)]
pub enum BackendMessage {
    LoadContacts,
    LoadMessages {
        contact: ContactId,
        start_ts: Bound<u64>,
        end_ts: Bound<u64>,
    },
    SendMessage(ContactId, MessageContent, Option<Quote>),
    DownloadAttachment(ContactId, u64, usize),
}

#[derive(Debug)]
pub enum FrontendMessage {
    LoadedContacts(Vec<Contact>),
    LoadedMessages(Vec<Message>),
    NewMessage(Message),
    DownloadedAttachment(ContactId, u64, usize, String),
    Tick,
}
