use super::{timestamp, Backend, ContactId};

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

    async fn background_sync(
        &mut self,
        _ba_tx: futures::channel::mpsc::UnboundedSender<crate::message::FrontendMessage>,
    ) -> super::Result<()> {
        std::future::pending::<()>().await;
        Ok(())
    }

    async fn users(&self) -> super::Result<Vec<super::Contact>> {
        Ok(vec![super::Contact {
            id: ContactId::User(vec![0]),
            name: "Self".to_owned(),
            address: "no address".to_owned(),
            last_message_timestamp: 0,
            description: "some description".to_owned(),
        }])
    }

    async fn groups(&self) -> super::Result<Vec<super::Contact>> {
        Ok(Vec::new())
    }

    async fn messages(
        &mut self,
        _contact: ContactId,
        _start_ts: std::ops::Bound<u64>,
        _end_ts: std::ops::Bound<u64>,
    ) -> super::Result<Vec<super::Message>> {
        let now = timestamp();
        let mut msgs = vec![
            super::Message {
                timestamp: now - 100,
                sender: vec![0],
                contact_id: ContactId::User(vec![0]),
                content: super::MessageContent::Text("Message 1".to_owned(), Vec::new()),
                quote: None,
            },
            super::Message {
                timestamp: now - 90,
                sender: vec![0],
                contact_id: ContactId::User(vec![0]),
                content: super::MessageContent::Text("Message 2".to_owned(), Vec::new()),
                quote: None,
            },
            super::Message {
                timestamp: now - 80,
                sender: vec![0],
                contact_id: ContactId::User(vec![0]),
                content: super::MessageContent::Reaction(
                    vec![0],
                    now - 100,
                    "🚀".to_owned(),
                    false,
                ),
                quote: None,
            },
        ];
        for i in (0..50).rev() {
            msgs.push(super::Message {
                timestamp: now - i,
                sender: vec![0],
                contact_id: ContactId::User(vec![0]),
                content: super::MessageContent::Text(format!("msg {i}"), Vec::new()),
                quote: None,
            });
        }
        Ok(msgs)
    }

    async fn send_message(
        &mut self,
        contact: ContactId,
        body: super::MessageContent,
        _quoted: Option<&super::Quote>,
    ) -> super::Result<super::Message> {
        let msg = super::Message {
            timestamp: timestamp(),
            sender: vec![0],
            contact_id: contact,
            content: body,
            quote: None,
        };
        Ok(msg)
    }

    async fn self_id(&self) -> Vec<u8> {
        vec![0]
    }

    async fn download_attachment(&self, _attachment_index: usize) -> super::Result<String> {
        Ok(String::new())
    }
}
