use std::path::PathBuf;

use presage::{libsignal_service::prelude::Uuid, store::Thread};

use super::{timestamp, Backend};

#[derive(Clone)]
pub struct Local {}

impl Backend for Local {
    async fn load(_path: &std::path::Path) -> super::Result<Self> {
        Ok(Self {})
    }

    async fn link(
        _path: &std::path::Path,
        _device_name: &str,
        _provisioning_link_tx: futures::channel::oneshot::Sender<url::Url>,
    ) -> super::Result<Self> {
        unimplemented!()
    }

    async fn sync_contacts(&mut self) -> super::Result<()> {
        Ok(())
    }

    async fn background_sync(
        &mut self,
        _ba_tx: futures::channel::mpsc::UnboundedSender<crate::message::FrontendMessage>,
    ) -> super::Result<()> {
        std::future::pending::<()>().await;
        Ok(())
    }

    async fn contacts(&self) -> super::Result<Vec<super::Contact>> {
        Ok(vec![super::Contact {
            thread_id: Thread::Contact(Uuid::nil()),
            name: "Self".to_owned(),
            address: "no address".to_owned(),
            last_message_timestamp: 0,
        }])
    }

    async fn groups(&self) -> super::Result<Vec<super::Contact>> {
        Ok(Vec::new())
    }

    async fn messages(
        &mut self,
        _contact: presage::store::Thread,
        _start_ts: std::ops::Bound<u64>,
        _end_ts: std::ops::Bound<u64>,
    ) -> super::Result<Vec<super::Message>> {
        let now = timestamp();
        Ok(vec![
            super::Message {
                timestamp: now - 100,
                sender: Uuid::nil(),
                thread: Thread::Contact(Uuid::nil()),
                content: super::MessageContent::Text("Message 1".to_owned(), Vec::new()),
            },
            super::Message {
                timestamp: now - 90,
                sender: Uuid::nil(),
                thread: Thread::Contact(Uuid::nil()),
                content: super::MessageContent::Text("Message 2".to_owned(), Vec::new()),
            },
            super::Message {
                timestamp: now - 80,
                sender: Uuid::nil(),
                thread: Thread::Contact(Uuid::nil()),
                content: super::MessageContent::Reaction(
                    Uuid::nil(),
                    now - 100,
                    "🚀".to_owned(),
                    false,
                ),
            },
        ])
    }

    async fn send_message(
        &mut self,
        contact: presage::store::Thread,
        body: super::MessageContent,
    ) -> super::Result<super::Message> {
        let msg = super::Message {
            timestamp: timestamp(),
            sender: Uuid::nil(),
            thread: contact,
            content: body,
        };
        Ok(msg)
    }

    async fn self_uuid(&self) -> presage::libsignal_service::prelude::Uuid {
        Uuid::nil()
    }

    async fn download_attachment(
        &self,
        _attachment_index: usize,
    ) -> super::Result<std::path::PathBuf> {
        Ok(PathBuf::new())
    }
}
