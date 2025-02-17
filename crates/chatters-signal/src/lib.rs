use chrono::Local;
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::pin_mut;
use futures::StreamExt;
use log::debug;
use log::info;
use log::warn;
use mime_guess::mime::APPLICATION_OCTET_STREAM;
use presage::libsignal_service::content::Content;
use presage::libsignal_service::content::ContentBody;
use presage::libsignal_service::prelude::Uuid;
use presage::libsignal_service::proto::data_message::Reaction;
use presage::libsignal_service::proto::DataMessage;
use presage::libsignal_service::protocol::ServiceId;
use presage::libsignal_service::sender::AttachmentSpec;
use presage::libsignal_service::zkgroup::GroupMasterKeyBytes;
use presage::proto::body_range::AssociatedValue;
use presage::proto::sync_message::Sent;
use presage::proto::AttachmentPointer;
use presage::proto::BodyRange;
use presage::proto::EditMessage;
use presage::proto::SyncMessage;
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

use chatters_lib::backends::timestamp;
use chatters_lib::backends::Backend;
use chatters_lib::backends::Contact;
use chatters_lib::backends::ContactId;
use chatters_lib::backends::Error;
use chatters_lib::backends::Message;
use chatters_lib::backends::MessageAttachment;
use chatters_lib::backends::MessageContent;
use chatters_lib::backends::Quote;
use chatters_lib::backends::Result;
use chatters_lib::message::FrontendMessage;

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
        let db_path = path.join("db");
        let config_store =
            SledStore::open(db_path, MigrationConflictStrategy::Raise, OnNewIdentity::Trust)
                .await
                .unwrap();

        let mut manager = match Manager::load_registered(config_store).await {
            Ok(manager) => manager,
            Err(_err) => return Err(Error::Unlinked),
        };

        let messages = manager.receive_messages().await.unwrap();
        pin_mut!(messages);
        while let Some(message) = messages.next().await {
            debug!(message:? = message; "Received message during load");
            match message {
                presage::model::messages::Received::QueueEmpty => break,
                presage::model::messages::Received::Contacts => continue,
                presage::model::messages::Received::Content(_) => {}
            }
        }

        let self_uuid = manager.whoami().await.unwrap().aci;
        let self_name = self_name(&mut manager).await;

        let attachments_dir = path.join("attachments");
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
        let db_path = path.join("db");
        let config_store =
            SledStore::open(db_path, MigrationConflictStrategy::Raise, OnNewIdentity::Trust)
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

        let attachments_dir = path.join("attachments");
        create_dir_all(&attachments_dir).unwrap();

        Ok(Self {
            manager,
            self_uuid,
            self_name,
            attachment_pointers: Vec::new(),
            attachments_dir,
        })
    }

    async fn background_sync(
        &mut self,
        ba_tx: mpsc::UnboundedSender<FrontendMessage>,
    ) -> Result<()> {
        loop {
            let messages = self.manager.receive_messages().await.unwrap();
            pin_mut!(messages);
            while let Some(message) = messages.next().await {
                debug!(message:? = message; "Received message during background_sync");
                match message {
                    presage::model::messages::Received::QueueEmpty => {}
                    presage::model::messages::Received::Contacts => {}
                    presage::model::messages::Received::Content(message) => {
                        if let Some((msg, attachment_pointers)) =
                            self.message_content_to_frontend_message(*message).await
                        {
                            self.attachment_pointers.extend(attachment_pointers);
                            ba_tx
                                .unbounded_send(FrontendMessage::NewMessage { message: msg })
                                .unwrap();
                        }
                    }
                }
            }
        }
    }

    async fn users(&self) -> Result<Vec<Contact>> {
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
                id: ContactId::User(contact.uuid.into_bytes().to_vec()),
                name,
                address: contact
                    .phone_number
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                last_message_timestamp,
                description: String::new(),
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
                id: ContactId::Group(key.to_vec()),
                name: group.title,
                address: String::new(),
                last_message_timestamp,
                description: group.description.unwrap_or_default(),
            });
        }
        Ok(ret)
    }

    async fn messages(
        &mut self,
        contact: ContactId,
        start_ts: Bound<u64>,
        end_ts: Bound<u64>,
    ) -> Result<Vec<Message>> {
        let mut ret = Vec::new();
        let thread = match contact {
            ContactId::User(vec) => Thread::Contact(Uuid::try_from(vec).unwrap()),
            ContactId::Group(vec) => Thread::Group(GroupMasterKeyBytes::try_from(vec).unwrap()),
        };
        let messages = self
            .manager
            .store()
            .messages(&thread, (start_ts, end_ts))
            .await
            .unwrap();
        for message in messages {
            match message {
                Ok(message) => {
                    if let Some((msg, attachment_pointers)) =
                        self.message_content_to_frontend_message(message).await
                    {
                        self.attachment_pointers.extend(attachment_pointers);
                        ret.push(msg)
                    }
                }
                Err(e) => {
                    warn!(error:% = e; "Failed to load message");
                }
            }
        }
        Ok(ret)
    }

    async fn send_message(
        &mut self,
        contact: ContactId,
        content: MessageContent,
        quoting: Option<&Quote>,
    ) -> Result<Message> {
        let now = timestamp();
        let quote = quoting.map(|q| {
            let sender = Uuid::try_from(q.sender.clone()).unwrap();
            presage::proto::data_message::Quote {
                id: Some(q.timestamp),
                author_aci: Some(sender.to_string()),
                text: Some(q.text.clone()),
                attachments: Vec::new(),
                body_ranges: Vec::new(),
                r#type: Some(presage::proto::data_message::quote::Type::Normal as i32),
            }
        });
        let content_body = match &content {
            MessageContent::Text { text, attachments } => {
                let attachments = if attachments.is_empty() {
                    Vec::new()
                } else {
                    self.upload_attachments(attachments).await
                };
                // TODO: copy attachments into local data dir if not already present
                ContentBody::DataMessage(DataMessage {
                    body: Some(text.clone()),
                    timestamp: Some(now),
                    quote,
                    attachments,
                    ..Default::default()
                })
            }
            MessageContent::Reaction {
                message_author,
                timestamp,
                reaction,
                remove,
            } => {
                let author = Uuid::try_from(message_author.clone()).unwrap();
                ContentBody::DataMessage(DataMessage {
                    reaction: Some(Reaction {
                        emoji: Some(reaction.clone()),
                        remove: Some(*remove),
                        target_author_aci: Some(author.to_string()),
                        target_sent_timestamp: Some(*timestamp),
                    }),
                    timestamp: Some(now),
                    quote,
                    ..Default::default()
                })
            }
            MessageContent::Edit {
                timestamp: _,
                text: _,
            } => {
                todo!()
            }
        };
        let quote = quoting.map(|quoted| Quote {
            timestamp: quoted.timestamp,
            sender: quoted.sender.clone(),
            text: quoted.text.clone(),
        });
        let ui_msg = Message {
            timestamp: now,
            sender: self.self_uuid.into_bytes().to_vec(),
            contact_id: contact.clone(),
            content,
            quote,
        };
        debug!(contact:? = contact, content:? = content_body; "Sending message");
        match contact {
            ContactId::User(id) => {
                let uuid = Uuid::try_from(id).unwrap();
                self.manager
                    .send_message(ServiceId::Aci(uuid.into()), content_body, now)
                    .await
                    .unwrap();
            }
            ContactId::Group(key) => {
                self.manager
                    .send_message_to_group(&key, content_body, now)
                    .await
                    .unwrap();
            }
        }
        Ok(ui_msg)
    }

    async fn self_id(&self) -> Vec<u8> {
        debug!("Getting self_uuid");
        self.manager
            .whoami()
            .await
            .unwrap()
            .aci
            .into_bytes()
            .to_vec()
    }

    async fn download_attachment(&self, attachment_index: usize) -> Result<PathBuf> {
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
            return Ok(file_path);
        }

        match std::fs::write(&file_path, &attachment_data) {
            Ok(()) => Ok(file_path),
            Err(e) => {
                warn!(error:% = e; "Failed to save attachment");
                Err(Error::Failure(format!("Failed to save attachment: {e}")))
            }
        }
    }
}

impl Signal {
    async fn last_message_timestamp(&self, thread_id: &Thread) -> Option<u64> {
        let messages = self
            .manager
            .store()
            .messages(thread_id, ..)
            .await
            .unwrap()
            .rev()
            .map(|m| m.unwrap());
        for msg in messages {
            if let Some((msg, _)) = self.message_content_to_frontend_message(msg).await {
                return Some(msg.timestamp);
            }
        }
        None
    }

    async fn message_content_to_frontend_message(
        &self,
        message: Content,
    ) -> Option<(Message, Vec<AttachmentPointer>)> {
        debug!(message:? = message; "Converting message to frontend message");
        let timestamp = message.metadata.timestamp;
        let thread = Thread::try_from(&message).unwrap();
        let sender = message.metadata.sender.raw_uuid();
        if let Some(m) = self
            .signal_message_to_message(timestamp, sender, thread, &message)
            .await
        {
            return Some(m);
        }
        debug!(message:? = message; "Unhandled backend message during conversion to frontend message");
        None
    }

    async fn signal_message_to_message(
        &self,
        timestamp: u64,
        sender: Uuid,
        thread: Thread,
        content: &Content,
    ) -> Option<(Message, Vec<AttachmentPointer>)> {
        match &content.body {
            ContentBody::DataMessage(dm) => {
                return self
                    .data_message_to_message(timestamp, sender, thread, dm)
                    .await
            }
            ContentBody::SynchronizeMessage(SyncMessage {
                sent: Some(Sent {
                    message: Some(dm), ..
                }),
                ..
            }) => {
                return self
                    .data_message_to_message(timestamp, sender, thread, dm)
                    .await
            }
            ContentBody::SynchronizeMessage(SyncMessage {
                sent:
                    Some(Sent {
                        edit_message:
                            Some(EditMessage {
                                target_sent_timestamp: Some(target_sent_timestamp),
                                data_message:
                                    Some(DataMessage {
                                        body: Some(text), ..
                                    }),
                            }),
                        ..
                    }),
                ..
            }) => {
                let msg = Message {
                    timestamp: *target_sent_timestamp,
                    sender: sender.into_bytes().to_vec(),
                    contact_id: match thread {
                        Thread::Contact(uuid) => ContactId::User(uuid.into_bytes().to_vec()),
                        Thread::Group(key) => ContactId::Group(key.to_vec()),
                    },
                    content: MessageContent::Edit {
                        timestamp,
                        text: text.clone(),
                    },
                    quote: None,
                };
                Some((msg, Vec::new()))
            }
            _ => None,
        }
    }

    async fn data_message_to_message(
        &self,
        timestamp: u64,
        sender: Uuid,
        thread: Thread,
        dm: &DataMessage,
    ) -> Option<(Message, Vec<AttachmentPointer>)> {
        let mut message = Message {
            timestamp,
            sender: sender.into_bytes().to_vec(),
            contact_id: match thread {
                Thread::Contact(uuid) => ContactId::User(uuid.into_bytes().to_vec()),
                Thread::Group(key) => ContactId::Group(key.to_vec()),
            },
            content: MessageContent::Text {
                text: String::new(),
                attachments: Vec::new(),
            },
            quote: None,
        };

        if dm.body.is_some() || !dm.attachments.is_empty() || dm.quote.is_some() {
            assert!(dm.reaction.is_none());
            let mut attachment_pointers = Vec::new();
            let attachments =
                dm.attachments
                    .iter()
                    .map(|attachment_pointer| {
                        let attachment_index = self.attachment_pointers.len();
                        let filename = attachment_pointer.file_name.clone().unwrap_or_else(|| {
                            Local::now().format("%Y-%m-%d-%H-%M-%s").to_string()
                        });
                        let size = attachment_pointer.size.unwrap() as u64;
                        attachment_pointers.push(attachment_pointer.clone());
                        let attachment_name = self.attachment_name(attachment_pointer);
                        let attachment_path = self.attachments_dir.join(&attachment_name);
                        let downloaded_path = if attachment_path.is_file() {
                            Some(attachment_path)
                        } else {
                            None
                        };
                        MessageAttachment {
                            name: filename,
                            index: attachment_index,
                            size,
                            path: downloaded_path,
                        }
                    })
                    .collect();

            let mut body = dm.body().to_owned();
            self.add_body_ranges(&mut body, &dm.body_ranges).await;

            message.content = MessageContent::Text {
                text: body,
                attachments,
            };
            if let Some(quote) = &dm.quote {
                let mut text = quote.text().to_owned();
                self.add_body_ranges(&mut text, &quote.body_ranges).await;
                let author_uuid: Uuid = quote.author_aci().parse().unwrap();
                message.quote = Some(Quote {
                    timestamp: quote.id(),
                    sender: author_uuid.into_bytes().to_vec(),
                    text,
                });
            }
            return Some((message, attachment_pointers));
        } else if let Some(r) = &dm.reaction {
            assert!(dm.body.is_none());
            assert!(dm.attachments.is_empty());
            let emoji = r.emoji.clone()?;
            let author_uuid: Uuid = r.target_author_aci.as_ref().unwrap().parse().unwrap();
            message.content = MessageContent::Reaction {
                message_author: author_uuid.into_bytes().to_vec(),
                timestamp: r.target_sent_timestamp.unwrap(),
                reaction: emoji,
                remove: r.remove(),
            };
            return Some((message, Vec::new()));
        }
        None
    }

    async fn add_body_ranges(&self, body: &mut String, ranges: &[BodyRange]) {
        for body_range in ranges {
            if let Some(AssociatedValue::MentionAci(aci)) = &body_range.associated_value {
                let user_uuid: Uuid = aci.parse().unwrap();
                if let Some(user) = self
                    .manager
                    .store()
                    .contact_by_id(&user_uuid)
                    .await
                    .unwrap()
                {
                    let username = format!("@{:?}", user.name);
                    let start = body_range.start.unwrap() as usize;
                    let end = start + body_range.length.unwrap() as usize;
                    let char_indices = body.char_indices().collect::<Vec<_>>();
                    let start_boundary = body.char_indices().nth(start).unwrap().0;
                    let end_boundary = body.char_indices().nth(end).unwrap().0;
                    debug!(body:?, start:?, end:?, username:?, char_indices:?, start_boundary:?, end_boundary:?; "Replacing body range");
                    body.replace_range(start_boundary..end_boundary, &username);
                }
            }
        }
    }

    fn attachment_name(&self, attachment_pointer: &AttachmentPointer) -> String {
        let hash: String = hex::encode(attachment_pointer.digest())
            .chars()
            .take(16)
            .collect();
        let mut filename = format!("{}-{}", attachment_pointer.upload_timestamp(), hash);
        if let Some(name) = &attachment_pointer.file_name {
            filename.push('-');
            filename.push_str(name)
        }

        let has_extension = attachment_pointer
            .file_name
            .as_ref()
            .map_or(false, |f| PathBuf::from(f).extension().is_some());
        if has_extension {
            return filename;
        }

        let extensions = mime_guess::get_mime_extensions_str(
            attachment_pointer
                .content_type
                .as_deref()
                .unwrap_or("application/octet-stream"),
        );
        let extension = extensions
            .and_then(|e| e.first())
            .map_or("bin", |v| v)
            .to_owned();
        format!("{filename}.{extension}")
    }

    async fn upload_attachments(
        &self,
        attachments: &[MessageAttachment],
    ) -> Vec<presage::proto::AttachmentPointer> {
        let attachment_specs: Vec<_> = attachments
            .iter()
            .map(|a| {
                let path = a.path.as_ref().unwrap();
                let data = std::fs::read(path).unwrap();
                (
                    AttachmentSpec {
                        content_type: mime_guess::from_path(path)
                            .first()
                            .unwrap_or(APPLICATION_OCTET_STREAM)
                            .to_string(),
                        length: data.len(),
                        file_name: path.file_name().map(|s| s.to_string_lossy().to_string()),
                        preview: None,
                        voice_note: None,
                        borderless: None,
                        width: None,
                        height: None,
                        caption: None,
                        blur_hash: None,
                    },
                    data,
                )
            })
            .collect();

        let attachments: std::result::Result<Vec<_>, _> = self
            .manager
            .upload_attachments(attachment_specs)
            .await
            .unwrap()
            .into_iter()
            .collect();

        attachments.unwrap()
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
