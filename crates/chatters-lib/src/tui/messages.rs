use std::collections::BTreeMap;

use ratatui::{text::Span, widgets::ListState};

use crate::backends::{ContactId, MessageAttachment};

use super::wrap_text;

#[derive(Debug, Clone)]
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
pub struct Message {
    pub timestamp: u64,
    pub sender: Vec<u8>,
    pub contact_id: ContactId,
    pub content: String,
    pub reactions: Vec<Reaction>,
    pub attachments: Vec<MessageAttachment>,
    pub quote: Option<Quote>,
    pub edits: Vec<MessageEdit>,
}

#[derive(Debug)]
pub struct MessageEdit {
    pub timestamp: u64,
    pub text: String,
}

impl Message {
    pub fn render(&self, width: usize) -> Vec<Span<'static>> {
        let mut lines = Vec::new();
        if let Some(quote) = &self.quote {
            if let Some(line) = quote.text.lines().next() {
                lines.push(Span::from(format!("> {line}")));
            }
        }
        if !self.attachments.is_empty() {
            for attachment in &self.attachments {
                lines.push(Span::from(attachment.message_line()));
            }
        }
        if let Some(edit) = self.edits.last() {
            let content = wrap_text(edit.text.trim(), width);
            for (i, line) in content.lines.iter().enumerate() {
                if i == 0 {
                    lines.push(Span::from(format!("e {line}")));
                } else {
                    lines.push(Span::from(format!("  {line}")));
                }
            }
        } else if !self.content.is_empty() {
            let content = wrap_text(self.content.trim(), width);
            for line in content.lines {
                lines.push(Span::from(format!("  {line}")));
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
            lines.push(Span::from(format!("r {}", react_line.join(" "))));
        }
        lines
    }
}
#[derive(Debug, Default)]
pub struct Messages {
    pub messages_by_ts: BTreeMap<u64, Message>,
    pub messages_by_index: Vec<u64>,
    pub timestamp_to_index: BTreeMap<u64, usize>,
    pub state: ListState,
}

impl Messages {
    pub fn add_single(&mut self, message: crate::backends::Message) {
        self.add_multiple(std::iter::once(message));
    }

    pub fn add_multiple(&mut self, messages: impl IntoIterator<Item = crate::backends::Message>) {
        for message in messages {
            match message.content {
                crate::backends::MessageContent::Text { text, attachments } => {
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
                            edits: Vec::new(),
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
                crate::backends::MessageContent::Edit {
                    timestamp: edit_timestamp,
                    text,
                } => {
                    let existing = self.messages_by_ts.get_mut(&message.timestamp).unwrap();
                    existing.edits.push(MessageEdit {
                        timestamp: edit_timestamp,
                        text,
                    });
                }
            }
        }
        self.messages_by_index = self.messages_by_ts.keys().copied().collect();
        self.timestamp_to_index = self
            .messages_by_index
            .iter()
            .enumerate()
            .map(|(i, ts)| (*ts, i))
            .collect();
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

    pub fn selected(&self) -> Option<&Message> {
        self.state.selected().and_then(|i| self.get_by_index(i))
    }

    pub fn select_message(&mut self, timestamp: u64) {
        let Some(index) = self.timestamp_to_index.get(&timestamp) else {
            return;
        };
        self.state.select(Some(*index));
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
