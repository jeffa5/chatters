use chrono::Local;
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::pin_mut;
use futures::StreamExt;
use log::debug;
use log::info;
use log::warn;
use presage::libsignal_service::content::Content;
use presage::libsignal_service::content::ContentBody;
use presage::libsignal_service::prelude::Uuid;
use presage::libsignal_service::proto::data_message::Reaction;
use presage::libsignal_service::proto::DataMessage;
use presage::libsignal_service::protocol::ServiceId;
use presage::proto::AttachmentPointer;
use presage::store::Thread;
use presage::{
    libsignal_service::configuration::SignalServers, manager::Registered,
    model::identity::OnNewIdentity, store::ContentsStore, Manager,
};
use presage_store_sled::{MigrationConflictStrategy, SledStore};
use std::fs::create_dir_all;
use std::ops::Bound;
use std::path::Path;
use std::path::PathBuf;
use url::Url;

use crate::backends::MessageAttachment;
use crate::message::FrontendMessage;

use super::timestamp;
use super::Backend;
use super::Contact;
use super::Error;
use super::Message;
use super::MessageContent;
use super::Result;

#[derive(Debug, Clone)]
pub struct Signal {
    manager: Manager<SledStore, Registered>,
    self_uuid: Uuid,
    self_name: String,
    attachment_pointers: Vec<AttachmentPointer>,
    attachments_dir: PathBuf,
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

        let attachments_dir = path.parent().unwrap().join("attachments");
        create_dir_all(&attachments_dir).unwrap();

        Ok(Signal {
            manager,
            self_uuid,
            self_name,
            attachment_pointers: Vec::new(),
            attachments_dir,
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

        let attachments_dir = path.parent().unwrap().join("attachments");
        create_dir_all(&attachments_dir).unwrap();

        Ok(Self {
            manager,
            self_uuid,
            self_name,
            attachment_pointers: Vec::new(),
            attachments_dir,
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
        &mut self,
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
            MessageContent::Text(t, _attachments) => ContentBody::DataMessage(DataMessage {
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

    async fn download_attachment(&self, attachment_index: usize) -> Result<String> {
        let Some(attachment_pointer) = self.attachment_pointers.get(attachment_index) else {
            return Err(Error::UnknownAttachment(attachment_index));
        };
        let Ok(attachment_data) = self.manager.get_attachment(attachment_pointer).await else {
            warn!(attachment:? = attachment_pointer; "failed to fetch attachment");
            return Err(Error::Failure("Failed to fetch attachment".to_owned()));
        };

        let file_name = self.attachment_name(attachment_pointer);
        let file_path = self.attachments_dir.join(&file_name);

        if file_path.is_file() {
            // already downloaded
            return Ok(file_name);
        }

        match std::fs::write(&file_path, &attachment_data) {
            Ok(()) => Ok(file_name),
            Err(e) => {
                warn!(error:% = e; "Failed to save attachment");
                Err(Error::Failure(format!("Failed to save attachment: {e}")))
            }
        }
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
                extract_data_message(m).map_or(false, |dm| {
                    !dm.attachments.is_empty() || dm.body.is_some() || dm.reaction.is_some()
                })
            })
            .next()
            .map(|m| m.metadata.timestamp)
            .unwrap_or_default()
    }

    fn message_content_to_frontend_message(&mut self, message: Content) -> Option<Message> {
        let thread = Thread::try_from(&message).unwrap();
        let sender = message.metadata.sender.raw_uuid();
        let data_message_to_message = |dm: &DataMessage| {
            let mut message = Message {
                timestamp: message.metadata.timestamp,
                sender,
                thread,
                content: MessageContent::Text(String::new(), Vec::new()),
            };

            if dm.body.is_some() || !dm.attachments.is_empty() {
                assert!(dm.reaction.is_none());
                let attachments = dm
                    .attachments
                    .iter()
                    .map(|attachment_pointer| {
                        let attachment_index = self.attachment_pointers.len();
                        let filename = attachment_pointer.file_name.clone().unwrap_or_else(|| {
                            Local::now().format("%Y-%m-%d-%H-%M-%s").to_string()
                        });
                        let size = attachment_pointer.size.unwrap();
                        self.attachment_pointers.push(attachment_pointer.clone());
                        let attachment_name = self.attachment_name(attachment_pointer);
                        let attachment_path = self.attachments_dir.join(&attachment_name);
                        let downloaded_path = if attachment_path.is_file() {
                            Some(attachment_name)
                        } else {
                            None
                        };
                        MessageAttachment {
                            name: filename,
                            size,
                            index: attachment_index,
                            downloaded_path,
                        }
                    })
                    .collect();
                let body = dm.body().to_owned();
                message.content = MessageContent::Text(body, attachments);
                return Some(message);
            } else if let Some(r) = &dm.reaction {
                assert!(dm.body.is_none());
                assert!(dm.attachments.is_empty());
                let emoji = r.emoji.clone()?;
                message.content = MessageContent::Reaction(
                    r.target_author_aci.as_ref().unwrap().parse().unwrap(),
                    r.target_sent_timestamp.unwrap(),
                    emoji,
                    r.remove(),
                );
                return Some(message);
            }
            None
        };
        if let Some(dm) = extract_data_message(&message) {
            if let Some(m) = data_message_to_message(dm) {
                return Some(m);
            }
        }
        debug!(message:? = message; "Unhandled backend message during conversion to frontend message");
        None
    }

    fn attachment_name(&self, attachment_pointer: &AttachmentPointer) -> String {
        let extensions = mime_guess::get_mime_extensions_str(
            attachment_pointer
                .content_type
                .as_deref()
                .unwrap_or("application/octet-stream"),
        );
        let extension = extensions.and_then(|e| e.first()).unwrap_or(&"bin");
        let hash: String = hex::encode(attachment_pointer.digest())
            .chars()
            .take(16)
            .collect();
        let mut filename = format!("{}-{}", attachment_pointer.upload_timestamp(), hash);
        if let Some(name) = &attachment_pointer.file_name {
            filename.push('-');
            filename.push_str(name)
        }
        format!("{filename}.{extension}")
    }
}

fn extract_data_message(content: &Content) -> Option<&DataMessage> {
    match &content.body {
        ContentBody::DataMessage(dm) => return Some(dm),
        ContentBody::SynchronizeMessage(sm) if sm.sent.is_some() => {
            if let Some(sent) = &sm.sent {
                if let Some(dm) = &sent.message {
                    return Some(dm);
                }
            }
        }
        _ => {}
    }
    None
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
