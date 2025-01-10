use std::collections::BTreeMap;

use presage::libsignal_service::prelude::Uuid;
use presage::store::Thread;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::style::Style;
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
use tui_input::Input;

use crate::backends::Contact;

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

#[derive(Debug, Default)]
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
    pub fn add(&mut self, message: crate::backends::Message) {
        match message.content {
            crate::backends::MessageContent::Text(content) => {
                // assume a new message
                self.messages_by_ts.insert(
                    message.timestamp,
                    Message {
                        timestamp: message.timestamp,
                        sender: message.sender,
                        thread: message.thread,
                        content,
                        reactions: Vec::new(),
                    },
                );
            }
            crate::backends::MessageContent::Reaction(author, ts, reaction, remove) => {
                if let Some(m) = self.messages_by_ts.get_mut(&ts) {
                    assert_eq!(m.sender, author);
                    let existing_reaction = m.reactions.iter().position(|r| r.author == author);
                    if let Some(existing_reaction) = existing_reaction {
                        if (remove && m.reactions[existing_reaction].emoji == reaction) || !remove {
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
        self.messages_by_index = self.messages_by_ts.keys().copied().collect();
    }

    pub fn get_by_index(&self, index: usize) -> Option<&Message> {
        self.messages_by_index
            .get(index)
            .and_then(|ts| self.messages_by_ts.get(ts))
    }

    pub fn clear(&mut self) {
        self.messages_by_ts.clear();
        self.messages_by_index.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.messages_by_ts.is_empty()
    }
}

impl FromIterator<crate::backends::Message> for Messages {
    fn from_iter<T: IntoIterator<Item = crate::backends::Message>>(iter: T) -> Self {
        let mut msgs = Self::default();
        for msg in iter {
            msgs.add(msg);
        }
        msgs
    }
}

#[derive(Debug)]
pub struct Message {
    pub timestamp: u64,
    pub sender: Uuid,
    pub thread: Thread,
    pub content: String,
    pub reactions: Vec<Reaction>,
}

#[derive(Debug)]
pub struct Reaction {
    pub author: Uuid,
    pub emoji: String,
}

#[derive(Debug, Default)]
pub struct TuiState {
    pub self_uuid: Uuid,
    pub contact_list_state: TableState,
    pub message_list_state: ListState,
    pub contacts: Vec<Contact>,
    pub contacts_by_id: BTreeMap<Uuid, Contact>,
    pub messages: Messages,
    pub compose: Input,
    pub command: Input,
    pub command_error: String,
    pub mode: Mode,
}

pub fn render(frame: &mut Frame<'_>, tui_state: &mut TuiState) {
    let now = timestamp();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(frame.area());
    let b = Block::default().borders(Borders::all());

    let main_rect =
        Layout::horizontal([Constraint::Percentage(25), Constraint::Fill(1)]).split(chunks[0]);

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
        .block(b.clone().title("Contacts"));
    frame.render_stateful_widget(contacts, main_rect[0], &mut tui_state.contact_list_state);

    let message_rect =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(main_rect[1]);

    let message_width = message_rect[0].width as usize;
    let message_items = tui_state.messages.messages_by_ts.values().map(|m| {
        let sender_width = 20;
        let age_width = 3;
        let content_width = message_width - sender_width - age_width - 2;
        let sender = tui_state
            .contacts_by_id
            .get(&m.sender)
            .map(|c| c.name.clone())
            .unwrap_or(m.sender.to_string());
        let sender = truncate_or_pad(sender, sender_width);
        let age = biggest_duration_string(now - m.timestamp);
        let content_indent = " ".repeat(message_width - content_width);
        let content = wrap_text(&m.content, content_width, message_width - content_width);
        let mut line = format!("{sender} {:>3} {content}", age);
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
            line.push('\n');
            line.push_str(&content_indent);
            line.push_str(&react_line.join(" "));
        }
        line
    });
    let messages = List::default()
        .items(message_items)
        .highlight_style(Style::new().reversed())
        .block(b.clone().title("Messages"));

    frame.render_stateful_widget(messages, message_rect[0], &mut tui_state.message_list_state);

    let compose_width = message_rect[1].width.max(3) - 3; // keep 2 for borders and 1 for cursor
    let compose_scroll = tui_state.compose.visual_scroll(compose_width as usize);
    let compose = Paragraph::new(tui_state.compose.value())
        .scroll((0, compose_scroll as u16))
        .block(b.clone().title("Compose"));
    frame.render_widget(compose, message_rect[1]);
    if matches!(tui_state.mode, Mode::Compose) {
        frame.set_cursor_position((
            // Put cursor past the end of the input text
            message_rect[1].x
                + ((tui_state.compose.visual_cursor()).max(compose_scroll) - compose_scroll) as u16
                + 1,
            // Move one line down, from the border to the input line
            message_rect[1].y + 1,
        ))
    }

    let status_line = Paragraph::new(Line::from(format!("mode:{:?}", tui_state.mode)))
        .block(b.clone().title("Status line"));
    frame.render_widget(status_line, chunks[1]);

    let (command_string, command_style) = if tui_state.command_error.is_empty() {
        (format!(":{}", tui_state.command.value()), Style::new())
    } else {
        (tui_state.command_error.clone(), Style::new().red())
    };
    let command_line = Paragraph::new(command_string)
        .style(command_style)
        .block(b.title("Exline"));
    frame.render_widget(command_line, chunks[2]);
    if matches!(tui_state.mode, Mode::Command) {
        frame.set_cursor_position((
            // Put cursor past the end of the input text
            chunks[2].x + tui_state.command.visual_cursor() as u16 + 2,
            // Move one line down, from the border to the input line
            chunks[2].y + 1,
        ))
    }
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

fn wrap_text(s: &str, width: usize, wrap_indent_width: usize) -> Text {
    let content = textwrap::wrap(
        &s,
        Options::new(width).subsequent_indent(&" ".repeat(wrap_indent_width)),
    )
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
