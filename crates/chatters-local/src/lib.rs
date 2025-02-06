use chatters_lib::backends::Contact;
use chatters_lib::backends::Message;
use chatters_lib::backends::MessageContent;
use chatters_lib::backends::Result;
use chatters_lib::backends::{timestamp, Backend, ContactId, Quote};
use chatters_lib::message::FrontendMessage;

#[derive(Clone)]
pub struct Local {}

impl Backend for Local {
    async fn load(_path: &std::path::Path) -> Result<Self> {
        Ok(Self {})
    }

    async fn link(
        _path: &std::path::Path,
        _device_name: &str,
        _provisioning_link_tx: futures::channel::oneshot::Sender<url::Url>,
    ) -> Result<Self> {
        unimplemented!()
    }

    async fn background_sync(
        &mut self,
        _ba_tx: futures::channel::mpsc::UnboundedSender<FrontendMessage>,
    ) -> Result<()> {
        std::future::pending::<()>().await;
        Ok(())
    }

    async fn users(&self) -> Result<Vec<Contact>> {
        Ok(vec![Contact {
            id: ContactId::User(vec![0]),
            name: "Self".to_owned(),
            address: "no address".to_owned(),
            last_message_timestamp: 0,
            description: "some description".to_owned(),
        }])
    }

    async fn groups(&self) -> Result<Vec<Contact>> {
        Ok(Vec::new())
    }

    async fn messages(
        &mut self,
        _contact: ContactId,
        _start_ts: std::ops::Bound<u64>,
        _end_ts: std::ops::Bound<u64>,
    ) -> Result<Vec<Message>> {
        let now = timestamp();
        let mut msgs = vec![
            Message {
                timestamp: now - 100,
                sender: vec![0],
                contact_id: ContactId::User(vec![0]),
                content: MessageContent::Text {
                    text: "Message 1".to_owned(),
                    attachments: Vec::new(),
                },
                quote: None,
            },
            Message {
                timestamp: now - 90,
                sender: vec![0],
                contact_id: ContactId::User(vec![0]),
                content: MessageContent::Text {
                    text: "Message 2".to_owned(),
                    attachments: Vec::new(),
                },
                quote: None,
            },
            Message {
                timestamp: now - 80,
                sender: vec![0],
                contact_id: ContactId::User(vec![0]),
                content: MessageContent::Reaction {
                    message_author: vec![0],
                    timestamp: now - 100,
                    reaction: "🚀".to_owned(),
                    remove: false,
                },
                quote: None,
            },
        ];
        for i in (0..50).rev() {
            msgs.push(Message {
                timestamp: now - i,
                sender: vec![0],
                contact_id: ContactId::User(vec![0]),
                content: MessageContent::Text {
                    text: format!("msg {i}"),
                    attachments: Vec::new(),
                },
                quote: None,
            });
        }
        Ok(msgs)
    }

    async fn send_message(
        &mut self,
        contact: ContactId,
        body: MessageContent,
        _quoted: Option<&Quote>,
    ) -> Result<Message> {
        let msg = Message {
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

    async fn download_attachment(&self, _attachment_index: usize) -> Result<String> {
        Ok(String::new())
    }
}
