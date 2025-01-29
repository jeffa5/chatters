use std::collections::BTreeMap;
use std::path::PathBuf;

use log::warn;
use presage::libsignal_service::prelude::Uuid;
use presage::store::Thread;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::style::Styled;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::List;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Row;
use ratatui::widgets::Table;
use ratatui::widgets::TableState;
use ratatui::Frame;
use textwrap::Options;
use tui_textarea::TextArea;

use crate::backends::Contact;

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

#[derive(Debug, Default, Clone, Copy)]
pub enum Mode {
    #[default]
    Normal,
    Command,
    Compose,
}

#[derive(Debug, Default)]
pub struct Messages {
    messages_by_ts: BTreeMap<u64, Message>,
    messages_by_index: Vec<u64>,
}

impl Messages {
    pub fn add_single(&mut self, message: crate::backends::Message) {
        self.add_multiple(std::iter::once(message));
    }

    pub fn add_multiple(&mut self, messages: impl IntoIterator<Item = crate::backends::Message>) {
        for message in messages {
            match message.content {
                crate::backends::MessageContent::Text(content, attachments) => {
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
                            thread: message.thread.clone(),
                            content,
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
                crate::backends::MessageContent::Reaction(author, ts, reaction, remove) => {
                    if let Some(m) = self.messages_by_ts.get_mut(&ts) {
                        assert_eq!(m.sender, author);
                        let existing_reaction = m.reactions.iter().position(|r| r.author == author);
                        if let Some(existing_reaction) = existing_reaction {
                            if (remove && m.reactions[existing_reaction].emoji == reaction)
                                || !remove
                            {
                                m.reactions.remove(existing_reaction);
                            }
                        }

                        if !remove {
                            m.reactions.push(Reaction {
                                author,
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

#[derive(Debug)]
pub struct Message {
    pub timestamp: u64,
    pub sender: Uuid,
    pub thread: Thread,
    pub content: String,
    pub reactions: Vec<Reaction>,
    pub attachments: Vec<Attachment>,
    pub quote: Option<Quote>,
}

#[derive(Debug)]
pub struct Quote {
    pub timestamp: u64,
    pub sender: Uuid,
    pub text: String,
}

#[derive(Debug)]
pub struct Reaction {
    pub author: Uuid,
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

#[derive(Debug, Default)]
pub struct TuiState {
    pub self_uuid: Uuid,
    pub attachments_path: String,
    pub contact_list_state: TableState,
    pub message_list_state: ListState,
    pub contacts: Vec<Contact>,
    pub contacts_by_id: BTreeMap<Uuid, Contact>,
    pub messages: Messages,
    pub compose: TextArea<'static>,
    pub command: TextArea<'static>,
    pub command_error: String,
    pub command_completions: Vec<String>,
    pub mode: Mode,
}

impl TuiState {
    pub fn selected_contact(&self) -> Option<&Contact> {
        self.contact_list_state
            .selected()
            .and_then(|i| self.contacts.get(i))
    }

    pub fn selected_message(&self) -> Option<&Message> {
        self.message_list_state
            .selected()
            .and_then(|i| self.messages.get_by_index(i))
    }
}

pub fn render(frame: &mut Frame<'_>, tui_state: &mut TuiState) {
    let now = timestamp();
    let vertical_splits = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let contacts_messages =
        Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(vertical_splits[0]);

    render_contacts(frame, contacts_messages[0], tui_state, now);

    let compose_height = tui_state.compose.lines().len().max(1) as u16 + 1; // 1 for top border
    let message_rect = Layout::vertical([Constraint::Fill(1), Constraint::Length(compose_height)])
        .split(contacts_messages[1]);

    render_messages(frame, message_rect[0], tui_state, now);
    render_compose(frame, message_rect[1], tui_state, now);

    render_status(frame, vertical_splits[1], tui_state, now);
    render_command(frame, vertical_splits[2], tui_state, now);
}

fn render_contacts(frame: &mut Frame<'_>, rect: Rect, tui_state: &mut TuiState, now: u64) {
    let contact_items = tui_state.contacts.iter().map(|c| {
        let age = if c.last_message_timestamp == 0 {
            String::new()
        } else {
            biggest_duration_string(now.saturating_sub(c.last_message_timestamp))
        };
        Row::new(vec![
            Text::from(c.name.to_string()),
            Text::from(age).alignment(Alignment::Right),
        ])
    });
    let contacts = Table::new(contact_items, [Constraint::Fill(1), Constraint::Length(3)])
        .row_highlight_style(Style::new().reversed())
        .block(Block::new().borders(Borders::RIGHT));
    frame.render_stateful_widget(contacts, rect, &mut tui_state.contact_list_state);
}

fn render_messages(frame: &mut Frame<'_>, rect: Rect, tui_state: &mut TuiState, now: u64) {
    let message_width = rect.width as usize;
    let message_items = tui_state.messages.messages_by_ts.values().map(|m| {
        let sender_width = 20;
        let sender = tui_state
            .contacts_by_id
            .get(&m.sender)
            .map(|c| c.name.clone())
            .unwrap_or(m.sender.to_string());
        let sender = truncate_or_pad(sender, sender_width);
        let age = biggest_duration_string(now.saturating_sub(m.timestamp));
        let sender_time = format!("{sender} {age:>3} ");

        let content_width = message_width - sender_time.len();
        let content_indent = " ".repeat(sender_time.len());

        let mut lines = Vec::new();
        if let Some(quote) = &m.quote {
            if let Some(line) = quote.text.lines().next() {
                lines.push(format!("> {line}"));
            }
        }
        if !m.content.is_empty() {
            let content = wrap_text(&m.content, content_width);
            for line in content.lines {
                lines.push(format!("  {line}"));
            }
        }
        if !m.attachments.is_empty() {
            for attachment in &m.attachments {
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
        if !m.reactions.is_empty() {
            let react_line = m
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

        for (i, line) in lines.iter_mut().enumerate() {
            if i == 0 {
                line.insert_str(0, &sender_time);
            } else {
                line.insert_str(0, &content_indent);
            }
        }
        if lines.is_empty() {
            warn!(message:? = m; "Message with no information...");
        }
        lines.join("\n")
    });
    let messages = List::default()
        .items(message_items)
        .highlight_style(Style::new().reversed());

    frame.render_stateful_widget(messages, rect, &mut tui_state.message_list_state);
}

fn render_compose(frame: &mut Frame<'_>, rect: Rect, tui_state: &mut TuiState, _now: u64) {
    tui_state
        .compose
        .set_block(Block::new().borders(Borders::TOP));
    if matches!(tui_state.mode, Mode::Compose) {
        // show cursor
        tui_state.compose.set_cursor_style(Style::new().reversed());
        tui_state
            .compose
            .set_cursor_line_style(Style::new().underlined());
    } else {
        // hide cursor
        tui_state.compose.set_cursor_style(Style::new());
        tui_state.compose.set_cursor_line_style(Style::new());
    }
    frame.render_widget(&tui_state.compose, rect);
}

fn render_status(frame: &mut Frame<'_>, rect: Rect, tui_state: &mut TuiState, _now: u64) {
    let completions = tui_state.command_completions.join(" ");
    let status_line = Paragraph::new(Line::from(format!("{:?} {}", tui_state.mode, completions)))
        .style(Style::new().reversed());
    frame.render_widget(status_line, rect);
}

fn render_command(frame: &mut Frame<'_>, rect: Rect, tui_state: &mut TuiState, _now: u64) {
    if tui_state.command_error.is_empty() {
        if matches!(tui_state.mode, Mode::Command) {
            let value = tui_state.command.lines().join("\n");
            (format!(":{}", value), Style::new());
            frame.render_widget(Line::from(":"), rect);
            let inner_rect = rect.inner(ratatui::layout::Margin {
                horizontal: 1,
                vertical: 0,
            });
            tui_state.command.set_cursor_line_style(Style::new());
            frame.render_widget(&tui_state.command, inner_rect);
        } else {
            frame.render_widget(ratatui::widgets::Clear::default(), rect);
        }
    } else {
        frame.render_widget(
            Paragraph::new(tui_state.command_error.clone()).set_style(Style::new().red()),
            rect,
        );
    };
}

fn biggest_duration_string(duration_ms: u64) -> String {
    let year = duration_ms / (1000 * 60 * 60 * 24 * 365);
    let month = duration_ms / (1000 * 60 * 60 * 24 * 30);
    let week = duration_ms / (1000 * 60 * 60 * 24 * 7);
    let day = duration_ms / (1000 * 60 * 60 * 24);
    let hour = duration_ms / (1000 * 60 * 60);
    let minute = duration_ms / (1000 * 60);
    let second = duration_ms / 1000;
    if year > 0 {
        format!("{year}y")
    } else if month > 0 {
        format!("{month}M")
    } else if week > 0 {
        format!("{week}w")
    } else if day > 0 {
        format!("{day}d")
    } else if hour > 0 {
        format!("{hour}h")
    } else if minute > 0 {
        format!("{minute}m")
    } else if second > 0 {
        format!("{second}s")
    } else {
        "now".to_owned()
    }
}

fn wrap_text(s: &str, width: usize) -> Text {
    let content = textwrap::wrap(&s, Options::new(width))
        .into_iter()
        .map(|s| Line::from(s.into_owned()))
        .collect::<Vec<_>>();
    Text::from(content)
}

fn truncate_or_pad(mut s: String, width: usize) -> String {
    if s.len() >= width {
        s[..width].to_owned()
    } else {
        s.push_str(&" ".repeat(width - s.len()));
        s
    }
}
