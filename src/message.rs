use std::ops::Bound;

use presage::store::Thread;

use crate::backends::{Contact, Message, MessageContent};

#[derive(Debug)]
pub enum BackendMessage {
    LoadContacts,
    LoadMessages {
        thread: Thread,
        start_ts: Bound<u64>,
        end_ts: Bound<u64>,
    },
    SendMessage(Thread, MessageContent),
}

#[derive(Debug)]
pub enum FrontendMessage {
    LoadedContacts(Vec<Contact>),
    LoadedMessages(Vec<Message>),
    NewMessage(Message),
    Tick,
}
