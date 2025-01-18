use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::pin_mut;
use futures::StreamExt;
use log::debug;
use log::info;
use presage::libsignal_service::content::Content;
use presage::libsignal_service::content::ContentBody;
use presage::libsignal_service::prelude::Uuid;
use presage::libsignal_service::proto::data_message::Reaction;
use presage::libsignal_service::proto::DataMessage;
use presage::libsignal_service::protocol::ServiceId;
use presage::store::Thread;
use presage::{
    libsignal_service::configuration::SignalServers, manager::Registered,
    model::identity::OnNewIdentity, store::ContentsStore, Manager,
};
use presage_store_sled::{MigrationConflictStrategy, SledStore};
use std::future::Future;
use std::ops::Bound;
use std::path::Path;
use url::Url;

use crate::message::FrontendMessage;

#[derive(Debug)]
pub struct Message {
    pub timestamp: u64,
    pub sender: Uuid,
    pub thread: Thread,
    pub content: MessageContent,
}

#[derive(Debug)]
pub enum MessageContent {
    Text(String),
    Reaction(Uuid, u64, String, bool),
}

#[derive(Debug, Clone)]
pub struct Contact {
    pub thread_id: Thread,
    pub name: String,
    pub address: String,
    pub last_message_timestamp: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unlinked")]
    Unlinked,
}

type Result<T> = std::result::Result<T, Error>;

pub trait Backend: Sized {
    fn load(path: &Path) -> impl Future<Output = Result<Self>>;

    fn link(
        path: &Path,
        device_name: &str,
        provisioning_link_tx: oneshot::Sender<Url>,
    ) -> impl Future<Output = Result<Self>>;

    fn sync_contacts(&mut self) -> impl Future<Output = Result<()>>;

    fn background_sync(
        &mut self,
        ba_tx: mpsc::UnboundedSender<FrontendMessage>,
    ) -> impl Future<Output = Result<()>>;

    fn contacts(&self) -> impl Future<Output = Result<Vec<Contact>>>;

    fn groups(&self) -> impl Future<Output = Result<Vec<Contact>>>;

    fn messages(
        &self,
        contact: Thread,
        start_ts: Bound<u64>,
        end_ts: Bound<u64>,
    ) -> impl Future<Output = Result<Vec<Message>>>;

    fn send_message(
        &mut self,
        contact: Thread,
        body: MessageContent,
    ) -> impl Future<Output = Result<Message>>;

    fn self_uuid(&self) -> impl Future<Output = Uuid>;
}

#[derive(Debug, Clone)]
pub struct Signal {
    manager: Manager<SledStore, Registered>,
    self_uuid: Uuid,
    self_name: String,
}

impl Backend for Signal {
    async fn load(path: &Path) -> Result<Self> {
        info!(path:? = path; "Loading signal backend");
        let config_store =
            SledStore::open(path, MigrationConflictStrategy::Raise, OnNewIdentity::Trust)
                .await
                .unwrap();

        let mut manager = match Manager::load_registered(config_store).await {
            Ok(manager) => manager,
            Err(_err) => return Err(Error::Unlinked),
        };

        let messages = manager.receive_messages().await.unwrap();
        pin_mut!(messages);
        while let Some(message) = messages.next().await {
            debug!(message:? = message; "Received message");
            match message {
                presage::model::messages::Received::QueueEmpty => break,
                presage::model::messages::Received::Contacts => continue,
                presage::model::messages::Received::Content(_) => {}
            }
        }

        let self_uuid = manager.whoami().await.unwrap().aci;
        let self_name = self_name(&mut manager).await;

        Ok(Signal {
            manager,
            self_uuid,
            self_name,
        })
    }

    async fn link(
        path: &Path,
        device_name: &str,
        provisioning_link_tx: oneshot::Sender<Url>,
    ) -> Result<Self> {
        let config_store =
            SledStore::open(path, MigrationConflictStrategy::Raise, OnNewIdentity::Trust)
                .await
                .unwrap();
        let mut manager = Manager::link_secondary_device(
            config_store,
            SignalServers::Production,
            device_name.to_owned(),
            provisioning_link_tx,
        )
        .await
        .unwrap();

        let self_uuid = manager.whoami().await.unwrap().aci;
        let self_name = self_name(&mut manager).await;

        Ok(Self {
            manager,
            self_uuid,
            self_name,
        })
    }

    async fn sync_contacts(&mut self) -> Result<()> {
        let messages = self.manager.receive_messages().await.unwrap();
        pin_mut!(messages);
        while let Some(message) = messages.next().await {
            debug!(message:? = message; "Received message");
            match message {
                presage::model::messages::Received::QueueEmpty => {}
                presage::model::messages::Received::Contacts => break,
                presage::model::messages::Received::Content(_) => {}
            }
        }
        Ok(())
    }

    async fn background_sync(
        &mut self,
        ba_tx: mpsc::UnboundedSender<FrontendMessage>,
    ) -> Result<()> {
        let messages = self.manager.receive_messages().await.unwrap();
        pin_mut!(messages);
        while let Some(message) = messages.next().await {
            debug!(message:? = message; "Received message");
            match message {
                presage::model::messages::Received::QueueEmpty => {}
                presage::model::messages::Received::Contacts => {}
                presage::model::messages::Received::Content(message) => {
                    if let Some(msg) = self.message_content_to_frontend_message(*message) {
                        ba_tx
                            .unbounded_send(FrontendMessage::NewMessage(msg))
                            .unwrap();
                    }
                }
            }
        }
        Ok(())
    }

    async fn contacts(&self) -> Result<Vec<Contact>> {
        let mut ret = Vec::new();
        let contacts = self.manager.store().contacts().await.unwrap();
        for contact in contacts {
            let contact = contact.unwrap();
            let name = if contact.uuid == self.self_uuid {
                self.self_name.clone()
            } else if contact.name.is_empty() {
                // skip contacts with no names
                continue;
            } else {
                contact.name.clone()
            };
            let last_message_timestamp = self
                .last_message_timestamp(&Thread::Contact(contact.uuid))
                .await;
            debug!(contact:? = contact; "Found contact");
            ret.push(Contact {
                thread_id: Thread::Contact(contact.uuid),
                name,
                address: contact
                    .phone_number
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                last_message_timestamp,
            });
        }
        Ok(ret)
    }

    async fn groups(&self) -> Result<Vec<Contact>> {
        let mut ret = Vec::new();
        let groups = self.manager.store().groups().await.unwrap();
        for group in groups {
            let (key, group) = group.unwrap();
            let last_message_timestamp = self.last_message_timestamp(&Thread::Group(key)).await;
            debug!(group:? = group; "Found group");
            ret.push(Contact {
                thread_id: Thread::Group(key),
                name: group.title,
                address: String::new(),
                last_message_timestamp,
            });
        }
        Ok(ret)
    }

    async fn messages(
        &self,
        contact: Thread,
        start_ts: Bound<u64>,
        end_ts: Bound<u64>,
    ) -> Result<Vec<Message>> {
        let mut ret = Vec::new();
        let messages = self
            .manager
            .store()
            .messages(&contact, (start_ts, end_ts))
            .await
            .unwrap();
        for message in messages {
            let message = message.unwrap();
            if let Some(msg) = self.message_content_to_frontend_message(message) {
                ret.push(msg)
            }
        }
        Ok(ret)
    }

    async fn send_message(&mut self, contact: Thread, content: MessageContent) -> Result<Message> {
        let now = timestamp();
        let content_body = match &content {
            MessageContent::Text(t) => ContentBody::DataMessage(DataMessage {
                body: Some(t.clone()),
                timestamp: Some(now),
                ..Default::default()
            }),
            MessageContent::Reaction(author, ts, r, remove) => {
                ContentBody::DataMessage(DataMessage {
                    reaction: Some(Reaction {
                        emoji: Some(r.clone()),
                        remove: Some(*remove),
                        target_author_aci: Some(author.to_string()),
                        target_sent_timestamp: Some(*ts),
                    }),
                    timestamp: Some(now),
                    ..Default::default()
                })
            }
        };
        let ui_msg = Message {
            timestamp: now,
            sender: self.self_uuid,
            thread: contact.clone(),
            content,
        };
        match contact {
            Thread::Contact(uuid) => {
                self.manager
                    .send_message(ServiceId::Aci(uuid.into()), content_body, now)
                    .await
                    .unwrap();
            }
            Thread::Group(key) => {
                self.manager
                    .send_message_to_group(&key, content_body, now)
                    .await
                    .unwrap();
            }
        }
        Ok(ui_msg)
    }

    async fn self_uuid(&self) -> Uuid {
        debug!("Getting self_uuid");
        self.manager.whoami().await.unwrap().aci
    }
}

impl Signal {
    async fn last_message_timestamp(&self, thread_id: &Thread) -> u64 {
        self.manager
            .store()
            .messages(thread_id, ..)
            .await
            .unwrap()
            .rev()
            .map(|m| m.unwrap())
            .filter(|m| {
                self.message_content_to_frontend_message(m.clone())
                    .is_some()
            })
            .next()
            .map(|m| m.metadata.timestamp)
            .unwrap_or_default()
    }

    fn message_content_to_frontend_message(&self, message: Content) -> Option<Message> {
        let thread = Thread::try_from(&message).unwrap();
        let data_message_to_message = |dm: &DataMessage, sender: Uuid| {
            if let Some(body) = &dm.body {
                return Some(Message {
                    timestamp: message.metadata.timestamp,
                    sender,
                    thread,
                    content: MessageContent::Text(body.clone()),
                });
            } else if let Some(r) = &dm.reaction {
                let emoji = r.emoji.clone()?;
                return Some(Message {
                    timestamp: message.metadata.timestamp,
                    sender,
                    thread,
                    content: MessageContent::Reaction(
                        r.target_author_aci.as_ref().unwrap().parse().unwrap(),
                        r.target_sent_timestamp.unwrap(),
                        emoji,
                        r.remove(),
                    ),
                });
            }
            None
        };
        match &message.body {
            ContentBody::DataMessage(dm) => {
                let sender = message.metadata.sender.raw_uuid();
                if let Some(m) = data_message_to_message(dm, sender) {
                    return Some(m);
                }
            }
            ContentBody::SynchronizeMessage(sm) if sm.sent.is_some() => {
                if let Some(sent) = &sm.sent {
                    if let Some(dm) = &sent.message {
                        if let Some(m) = data_message_to_message(dm, self.self_uuid) {
                            return Some(m);
                        }
                    }
                }
            }
            _ => {}
        }
        debug!(message:? = message; "Unhandled backend message during conversion to frontend message");
        None
    }
}

async fn self_name(manager: &mut Manager<SledStore, Registered>) -> String {
    manager
        .retrieve_profile()
        .await
        .unwrap()
        .name
        .map(|n| {
            if let Some(f) = n.family_name {
                format!("{} {}", n.given_name, f)
            } else {
                n.given_name
            }
        })
        .unwrap_or("Self".to_owned())
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}
