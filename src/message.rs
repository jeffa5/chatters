use presage::store::Thread;

use crate::backends::{Contact, Message, MessageContent};

#[derive(Debug)]
pub enum BackendMessage {
    LoadContacts,
    LoadMessages(Thread),
    SendMessage(Thread, MessageContent),
}

#[derive(Debug)]
pub enum FrontendMessage {
    LoadedContacts(Vec<Contact>),
    LoadedMessages(Vec<Message>),
    NewMessage(Message),
    Tick,
}
