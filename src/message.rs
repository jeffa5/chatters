use presage::store::Thread;

use crate::backends::{Contact, Message};

#[derive(Debug)]
pub enum BackendMessage {
    LoadContacts,
    LoadMessages(Thread),
    SendMessage(Thread, String),
}

#[derive(Debug)]
pub enum FrontendMessage {
    LoadedContacts(Vec<Contact>),
    LoadedMessages(Vec<Message>),
    NewMessage(Message),
    Tick,
}
