use std::path::PathBuf;

use presage::libsignal_service::prelude::Uuid;

use super::Backend;

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
        Ok(Self {})
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
        Ok(Vec::new())
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
        Ok(Vec::new())
    }

    async fn send_message(
        &mut self,
        contact: presage::store::Thread,
        body: super::MessageContent,
    ) -> super::Result<super::Message> {
        Ok(super::Message {
            timestamp: 0,
            sender: Uuid::default(),
            thread: contact,
            content: body,
        })
    }

    async fn self_uuid(&self) -> presage::libsignal_service::prelude::Uuid {
        Uuid::default()
    }

    async fn download_attachment(
        &self,
        _attachment_index: usize,
    ) -> super::Result<std::path::PathBuf> {
        Ok(PathBuf::new())
    }
}
