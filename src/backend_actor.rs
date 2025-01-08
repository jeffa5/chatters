use std::cmp::Reverse;

use crate::{
    backends::Backend,
    message::{BackendMessage, FrontendMessage},
};
use futures::StreamExt;

pub struct BackendActor<B> {
    pub backend: B,
    pub message_rx: futures::channel::mpsc::UnboundedReceiver<BackendMessage>,
    pub message_tx: futures::channel::mpsc::UnboundedSender<FrontendMessage>,
}

impl<B: Backend> BackendActor<B> {
    pub async fn run(&mut self) {
        eprintln!("Started backend actor");
        while let Some(message) = self.message_rx.next().await {
            match message {
                BackendMessage::LoadContacts => {
                    let mut contacts = self.backend.contacts().await.unwrap();
                    let mut groups = self.backend.groups().await.unwrap();
                    contacts.append(&mut groups);
                    contacts.sort_by_key(|c| (Reverse(c.last_message_timestamp), c.name.clone()));
                    self.message_tx
                        .unbounded_send(FrontendMessage::LoadedContacts(contacts))
                        .unwrap();
                }
                BackendMessage::LoadMessages(thread) => {
                    let messages = self.backend.messages(thread).await.unwrap();
                    self.message_tx
                        .unbounded_send(FrontendMessage::LoadedMessages(messages))
                        .unwrap();
                }
                BackendMessage::SendMessage(thread, body) => {
                    let msg = self.backend.send_message(thread, body).await.unwrap();
                    self.message_tx
                        .unbounded_send(FrontendMessage::NewMessage(msg))
                        .unwrap();
                }
            }
        }
        eprintln!("Closing backend actor");
    }
}
