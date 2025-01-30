use std::ops::Bound;

use presage::store::Thread;

use crate::backends::{Contact, Message, MessageContent, Quote};

#[derive(Debug)]
pub enum BackendMessage {
    LoadContacts,
    LoadMessages {
        thread: Thread,
        start_ts: Bound<u64>,
        end_ts: Bound<u64>,
    },
    SendMessage(Thread, MessageContent, Option<Quote>),
    DownloadAttachment(Thread, u64, usize),
}

#[derive(Debug)]
pub enum FrontendMessage {
    LoadedContacts(Vec<Contact>),
    LoadedMessages(Vec<Message>),
    NewMessage(Message),
    DownloadedAttachment(Thread, u64, usize, String),
    Tick,
}
