use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::pin_mut;
use futures::StreamExt;
use presage::libsignal_service::content::Content;
use presage::libsignal_service::content::ContentBody;
use presage::libsignal_service::prelude::Uuid;
use presage::libsignal_service::proto::DataMessage;
use presage::libsignal_service::protocol::ServiceId;
use presage::store::Thread;
use presage::{
    libsignal_service::configuration::SignalServers, manager::Registered,
    model::identity::OnNewIdentity, store::ContentsStore, Manager,
};
use presage_store_sled::{MigrationConflictStrategy, SledStore};
use std::future::Future;
use std::path::Path;
use std::time::SystemTime;
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
    Reaction(u64, String, bool),
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

    fn messages(&self, contact: Thread) -> impl Future<Output = Result<Vec<Message>>>;

    fn send_message(
        &mut self,
        contact: Thread,
        body: String,
    ) -> impl Future<Output = Result<Message>>;
}

#[derive(Debug, Clone)]
pub struct Signal {
    manager: Manager<SledStore, Registered>,
    self_uuid: Uuid,
    self_name: String,
}

impl Backend for Signal {
    async fn load(path: &Path) -> Result<Self> {
        let config_store =
            SledStore::open(path, MigrationConflictStrategy::Raise, OnNewIdentity::Trust)
                .await
                .unwrap();

        let mut manager = match Manager::load_registered(config_store).await {
            Ok(manager) => manager,
            Err(_err) => return Err(Error::Unlinked),
        };

        let messages = manager
            .receive_messages(presage::manager::ReceivingMode::InitialSync)
            .await
            .unwrap();
        pin_mut!(messages);
        while let Some(message) = messages.next().await {
            eprintln!("Received message {message:?}");
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
        self.manager.sync_contacts().await.unwrap();
        Ok(())
    }

    async fn background_sync(
        &mut self,
        ba_tx: mpsc::UnboundedSender<FrontendMessage>,
    ) -> Result<()> {
        let messages = self
            .manager
            .receive_messages(presage::manager::ReceivingMode::Forever)
            .await
            .unwrap();
        pin_mut!(messages);
        while let Some(message) = messages.next().await {
            eprintln!("Received message {message:?}");
            if let Some(msg) = self.message_content_to_frontend_message(message) {
                ba_tx
                    .unbounded_send(FrontendMessage::NewMessage(msg))
                    .unwrap();
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
            eprintln!("{:?}", contact);
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
            eprintln!("{:?}", group);
            ret.push(Contact {
                thread_id: Thread::Group(key),
                name: group.title,
                address: String::new(),
                last_message_timestamp,
            });
        }
        Ok(ret)
    }

    async fn messages(&self, contact: Thread) -> Result<Vec<Message>> {
        let mut ret = Vec::new();
        let one_day_seconds = 60 * 60 * 24;
        let one_day_ago = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - one_day_seconds;
        let messages = self
            .manager
            .store()
            .messages(&contact, one_day_ago..)
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

    async fn send_message(&mut self, contact: Thread, body: String) -> Result<Message> {
        let ts = timestamp();
        let content_body = ContentBody::DataMessage(DataMessage {
            body: Some(body.clone()),
            timestamp: Some(ts),
            ..Default::default()
        });
        let ui_msg = Message {
            timestamp: ts,
            sender: self.self_uuid,
            thread: contact.clone(),
            content: MessageContent::Text(body),
        };
        match contact {
            Thread::Contact(uuid) => {
                self.manager
                    .send_message(ServiceId::Aci(uuid.into()), content_body, ts)
                    .await
                    .unwrap();
            }
            Thread::Group(key) => {
                self.manager
                    .send_message_to_group(&key, content_body, ts)
                    .await
                    .unwrap();
            }
        }
        Ok(ui_msg)
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
        match &message.body {
            ContentBody::DataMessage(dm) => {
                let sender = message.metadata.sender.raw_uuid();
                if let Some(body) = &dm.body {
                    return Some(Message {
                        timestamp: message.metadata.timestamp,
                        sender,
                        thread,
                        content: MessageContent::Text(body.clone()),
                    });
                } else if let Some(r) = &dm.reaction {
                    return Some(Message {
                        timestamp: message.metadata.timestamp,
                        sender,
                        thread,
                        content: MessageContent::Reaction(
                            r.target_sent_timestamp.unwrap(),
                            r.emoji.clone().unwrap(),
                            r.remove(),
                        ),
                    });
                }
            }
            ContentBody::SynchronizeMessage(sm) if sm.sent.is_some() => {
                if let Some(sent) = &sm.sent {
                    if let Some(dm) = &sent.message {
                        if let Some(body) = &dm.body {
                            return Some(Message {
                                timestamp: sent.timestamp(),
                                sender: self.self_uuid,
                                thread,
                                content: MessageContent::Text(body.clone()),
                            });
                        } else if let Some(r) = &dm.reaction {
                            return Some(Message {
                                timestamp: message.metadata.timestamp,
                                sender: self.self_uuid,
                                thread,
                                content: MessageContent::Reaction(
                                    r.target_sent_timestamp.unwrap(),
                                    r.emoji.clone().unwrap(),
                                    r.remove(),
                                ),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
        dbg!(&message);
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
