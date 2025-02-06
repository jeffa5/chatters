use std::{collections::BTreeMap, path::PathBuf};

use crate::backends::ContactId;

use super::wrap_text;

#[derive(Debug)]
pub struct Quote {
    pub timestamp: u64,
    pub sender: Vec<u8>,
    pub text: String,
}

#[derive(Debug)]
pub struct Reaction {
    pub author: Vec<u8>,
    pub emoji: String,
}

#[derive(Debug)]
pub struct Attachment {
    pub name: String,
    pub size: u32,
    pub handle: usize,
    pub downloaded_file_name: Option<String>,
    pub downloaded_file_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct Message {
    pub timestamp: u64,
    pub sender: Vec<u8>,
    pub contact_id: ContactId,
    pub content: String,
    pub reactions: Vec<Reaction>,
    pub attachments: Vec<Attachment>,
    pub quote: Option<Quote>,
}

impl Message {
    pub fn render(&self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(quote) = &self.quote {
            if let Some(line) = quote.text.lines().next() {
                lines.push(format!("> {line}"));
            }
        }
        if !self.content.is_empty() {
            let content = wrap_text(&self.content, width);
            for line in content.lines {
                lines.push(format!("  {line}"));
            }
        }
        if !self.attachments.is_empty() {
            for attachment in &self.attachments {
                let downloaded = attachment
                    .downloaded_file_name
                    .clone()
                    .unwrap_or_else(|| "not downloaded".to_owned());
                lines.push(format!(
                    "+ {} {}B ({})",
                    attachment.name, attachment.size, downloaded
                ));
            }
        }
        if !self.reactions.is_empty() {
            let react_line = self
                .reactions
                .iter()
                .fold(BTreeMap::<_, usize>::new(), |mut map, r| {
                    *map.entry(&r.emoji).or_default() += 1;
                    map
                })
                .iter()
                .map(|(r, count)| {
                    if *count > 1 {
                        format!("{r}x{count}")
                    } else {
                        (*r).to_owned()
                    }
                })
                .collect::<Vec<_>>();
            lines.push(format!("  {}", react_line.join(" ")));
        }
        lines
    }
}
#[derive(Debug, Default)]
pub struct Messages {
    pub messages_by_ts: BTreeMap<u64, Message>,
    pub messages_by_index: Vec<u64>,
}

impl Messages {
    pub fn add_single(&mut self, message: crate::backends::Message) {
        self.add_multiple(std::iter::once(message));
    }

    pub fn add_multiple(&mut self, messages: impl IntoIterator<Item = crate::backends::Message>) {
        for message in messages {
            match message.content {
                crate::backends::MessageContent::Text { text, attachments } => {
                    let attachments = attachments
                        .into_iter()
                        .map(|a| Attachment {
                            name: a.name,
                            size: a.size,
                            handle: a.index,
                            downloaded_file_name: a.downloaded_name,
                            downloaded_file_path: a.downloaded_path,
                        })
                        .collect();
                    // assume a new message
                    self.messages_by_ts.insert(
                        message.timestamp,
                        Message {
                            timestamp: message.timestamp,
                            sender: message.sender,
                            contact_id: message.contact_id.clone(),
                            content: text,
                            reactions: Vec::new(),
                            attachments,
                            quote: message.quote.map(|q| Quote {
                                timestamp: q.timestamp,
                                sender: q.sender,
                                text: q.text,
                            }),
                        },
                    );
                }
                crate::backends::MessageContent::Reaction {
                    message_author,
                    timestamp,
                    reaction,
                    remove,
                } => {
                    if let Some(m) = self.messages_by_ts.get_mut(&timestamp) {
                        assert_eq!(m.sender, message_author);
                        let existing_reaction =
                            m.reactions.iter().position(|r| r.author == message_author);
                        if let Some(existing_reaction) = existing_reaction {
                            if (remove && m.reactions[existing_reaction].emoji == reaction)
                                || !remove
                            {
                                m.reactions.remove(existing_reaction);
                            }
                        }

                        if !remove {
                            m.reactions.push(Reaction {
                                author: message_author,
                                emoji: reaction,
                            });
                        }
                    }
                }
            }
        }
        self.messages_by_index = self.messages_by_ts.keys().copied().collect();
    }

    pub fn get_by_index(&self, index: usize) -> Option<&Message> {
        self.messages_by_index
            .get(index)
            .and_then(|ts| self.messages_by_ts.get(ts))
    }

    pub fn get_by_timestamp(&self, timestamp: u64) -> Option<&Message> {
        self.messages_by_ts.get(&timestamp)
    }

    pub fn get_mut_by_timestamp(&mut self, timestamp: u64) -> Option<&mut Message> {
        self.messages_by_ts.get_mut(&timestamp)
    }

    pub fn clear(&mut self) {
        self.messages_by_ts.clear();
        self.messages_by_index.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.messages_by_ts.is_empty()
    }

    pub fn len(&self) -> usize {
        self.messages_by_ts.len()
    }
}

impl FromIterator<crate::backends::Message> for Messages {
    fn from_iter<T: IntoIterator<Item = crate::backends::Message>>(iter: T) -> Self {
        let mut msgs = Self::default();
        msgs.extend(iter);
        msgs
    }
}

impl Extend<crate::backends::Message> for Messages {
    fn extend<T: IntoIterator<Item = crate::backends::Message>>(&mut self, iter: T) {
        self.add_multiple(iter);
    }
}
