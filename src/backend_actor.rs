use std::cmp::Reverse;

use crate::{
    backends::Backend,
    message::{BackendMessage, FrontendMessage},
};
use futures::StreamExt;
use log::info;

pub struct BackendActor<B> {
    pub backend: B,
    pub message_rx: futures::channel::mpsc::UnboundedReceiver<BackendMessage>,
    pub message_tx: futures::channel::mpsc::UnboundedSender<FrontendMessage>,
}

impl<B: Backend> BackendActor<B> {
    pub async fn run(&mut self) {
        info!("Started backend actor");
        while let Some(message) = self.message_rx.next().await {
            match message {
                BackendMessage::LoadContacts => {
                    let mut contacts = self.backend.users().await.unwrap();
                    let mut groups = self.backend.groups().await.unwrap();
                    contacts.append(&mut groups);
                    contacts.sort_by_key(|c| (Reverse(c.last_message_timestamp), c.name.clone()));
                    self.message_tx
                        .unbounded_send(FrontendMessage::LoadedContacts(contacts))
                        .unwrap();
                }
                BackendMessage::LoadMessages {
                    contact,
                    start_ts,
                    end_ts,
                } => {
                    let messages = self
                        .backend
                        .messages(contact, start_ts, end_ts)
                        .await
                        .unwrap();
                    self.message_tx
                        .unbounded_send(FrontendMessage::LoadedMessages(messages))
                        .unwrap();
                }
                BackendMessage::SendMessage(contact, body, quoted) => {
                    let msg = self
                        .backend
                        .send_message(contact, body, quoted.as_ref())
                        .await
                        .unwrap();
                    self.message_tx
                        .unbounded_send(FrontendMessage::NewMessage(msg))
                        .unwrap();
                }
                BackendMessage::DownloadAttachment(contact, timestamp, index) => {
                    let file_name = self.backend.download_attachment(index).await.unwrap();
                    self.message_tx
                        .unbounded_send(FrontendMessage::DownloadedAttachment(
                            contact, timestamp, index, file_name,
                        ))
                        .unwrap();
                }
            }
        }
        info!("Closing backend actor");
    }
}
