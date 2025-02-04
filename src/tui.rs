use std::collections::BTreeMap;
use std::path::PathBuf;

use log::warn;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Flex;
use ratatui::layout::Layout;
use ratatui::layout::Margin;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::style::Styled;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Row;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::Table;
use ratatui::widgets::TableState;
use ratatui::Frame;
use textwrap::Options;
use tui_textarea::TextArea;

use crate::backends::Contact;
use crate::backends::ContactId;
use crate::command_history::CommandLineHistory;
use crate::compose::Compose;
use crate::contacts::Contacts;
use crate::keybinds::KeyBinds;

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
    Popup,
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
                            contact: message.contact_id.clone(),
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

#[derive(Debug)]
pub struct Message {
    pub timestamp: u64,
    pub sender: Vec<u8>,
    pub contact: ContactId,
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
pub enum Popup {
    MessageInfo { timestamp: u64 },
    ContactInfo { id: ContactId },
    Keybinds,
    Commands,
    CommandHistory,
}

#[derive(Debug, Default)]
pub struct TuiState {
    pub self_id: Vec<u8>,
    pub attachments_path: String,
    pub contact_list_state: TableState,
    pub message_list_state: ListState,
    pub contacts: Contacts,
    pub messages: Messages,
    pub compose: Compose,
    pub command: TextArea<'static>,
    pub command_error: String,
    pub command_completions: Vec<String>,
    pub command_history: CommandLineHistory,
    pub mode: Mode,
    pub popup: Option<Popup>,
    pub popup_scroll: u16,
}

impl TuiState {
    pub fn selected_contact(&self) -> Option<&Contact> {
        self.contact_list_state
            .selected()
            .and_then(|i| self.contacts.contact_or_group_by_index(i))
    }

    pub fn selected_message(&self) -> Option<&Message> {
        self.message_list_state
            .selected()
            .and_then(|i| self.messages.get_by_index(i))
    }
}

pub fn render(frame: &mut Frame<'_>, tui_state: &mut TuiState) {
    let now = timestamp();
    let area = frame.area();
    let vertical_splits = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let contacts_messages =
        Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(vertical_splits[0]);

    render_contacts(frame, contacts_messages[0], tui_state, now);

    let compose_height = tui_state.compose.height();
    let message_rect = Layout::vertical([Constraint::Fill(1), Constraint::Length(compose_height)])
        .split(contacts_messages[1]);

    render_messages(frame, message_rect[0], tui_state, now);
    render_compose(frame, message_rect[1], tui_state, now);

    render_status(frame, vertical_splits[1], tui_state, now);
    render_command(frame, vertical_splits[2], tui_state, now);

    render_popup(frame, area, tui_state);
}

fn render_contacts(frame: &mut Frame<'_>, rect: Rect, tui_state: &mut TuiState, now: u64) {
    let contact_items: Vec<_> = tui_state
        .contacts
        .iter_contacts_and_groups()
        .map(|c| {
            let age = if c.last_message_timestamp == 0 {
                String::new()
            } else {
                biggest_duration_string(now.saturating_sub(c.last_message_timestamp))
            };
            Row::new(vec![
                Text::from(c.name.to_string()),
                Text::from(age).alignment(Alignment::Right),
            ])
        })
        .collect();
    let contact_items_len = contact_items.len();
    let contacts = Table::new(contact_items, [Constraint::Fill(1), Constraint::Length(3)])
        .row_highlight_style(Style::new().reversed())
        .block(Block::new().borders(Borders::RIGHT));
    frame.render_stateful_widget(
        contacts,
        Rect {
            // leave room for the scrollbar
            width: rect.width.saturating_sub(1),
            ..rect
        },
        &mut tui_state.contact_list_state,
    );

    render_scrollbar(
        frame,
        rect,
        contact_items_len,
        tui_state.contact_list_state.offset(),
    );
}

fn render_messages(frame: &mut Frame<'_>, rect: Rect, tui_state: &mut TuiState, now: u64) {
    let message_width = rect.width as usize;
    let message_items = tui_state.messages.messages_by_ts.values().map(|m| {
        let sender_width = 20;
        let sender = tui_state
            .contacts
            .contact_by_id(&m.sender)
            .map(|c| c.name.clone())
            .unwrap();
        let sender = truncate_or_pad(sender, sender_width);
        let age = biggest_duration_string(now.saturating_sub(m.timestamp));
        let sender_time = format!("{sender} {age:>3} ");

        let content_width = message_width
            .saturating_sub(sender_time.len())
            .saturating_sub(1);
        let content_indent = " ".repeat(sender_time.len());

        let mut lines = m.render(content_width);

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

    frame.render_stateful_widget(
        &messages,
        Rect {
            // leave room for the scrollbar
            width: rect.width.saturating_sub(1),
            ..rect
        },
        &mut tui_state.message_list_state,
    );

    render_scrollbar(
        frame,
        rect,
        messages.len(),
        tui_state.message_list_state.offset(),
    );
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
            frame.render_widget(ratatui::widgets::Clear, rect);
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
    let content = textwrap::wrap(s, Options::new(width))
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

fn render_popup(frame: &mut Frame<'_>, area: Rect, tui_state: &mut TuiState) {
    let Some(popup) = &tui_state.popup else {
        return;
    };
    let area = popup_area(area, 60, 50);
    frame.render_widget(Clear, area); // this clears out the background
    let width = area.width.saturating_sub(2) as usize;
    let (title, text) = match popup {
        Popup::MessageInfo { timestamp } => {
            let message = tui_state.messages.get_by_timestamp(*timestamp).unwrap();
            render_message_info(width, tui_state, message)
        }
        Popup::ContactInfo { id } => {
            let contact = tui_state
                .contacts
                .iter_contacts_and_groups()
                .find(|c| &c.id == id)
                .unwrap();
            render_contact_info(contact)
        }
        Popup::Keybinds => render_keybinds(),
        Popup::Commands => render_commands(),
        Popup::CommandHistory => render_command_line_history(tui_state),
    };

    let text = wrap_text(&text, width);

    let line_count = text.lines.len() as u16;
    let max_scroll = line_count.saturating_sub(area.height.saturating_sub(2));
    tui_state.popup_scroll = tui_state.popup_scroll.min(max_scroll);
    let block = Block::bordered().title(title);
    let para = Paragraph::new(text)
        .block(block)
        .scroll((tui_state.popup_scroll, 0));
    frame.render_widget(para, area);
    render_scrollbar(
        frame,
        area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        line_count.into(),
        tui_state.popup_scroll.into(),
    );
}

fn render_message_info(
    width: usize,
    tui_state: &TuiState,
    message: &Message,
) -> (&'static str, String) {
    let ts_seconds = message.timestamp / 1_000;
    let ts_nanos = (message.timestamp % 1_000) * 1_000_000;
    let time = chrono::DateTime::from_timestamp(
        ts_seconds.try_into().unwrap(),
        ts_nanos.try_into().unwrap(),
    )
    .unwrap();
    let sender_name = tui_state
        .contacts
        .contact_by_id(&message.sender)
        .unwrap()
        .name
        .to_string();
    let text = vec![
        format!("Sender name: {}", sender_name),
        format!("Sender id:   {:?}", message.sender),
        format!("Time:        {}", time.to_rfc3339()),
        String::new(),
        message.render(width).join("\n"),
    ]
    .join("\n");
    ("Message info", text)
}

fn render_contact_info(contact: &Contact) -> (&'static str, String) {
    let ts_seconds = contact.last_message_timestamp / 1_000;
    let ts_nanos = (contact.last_message_timestamp % 1_000) * 1_000_000;
    let time = chrono::DateTime::from_timestamp(
        ts_seconds.try_into().unwrap(),
        ts_nanos.try_into().unwrap(),
    )
    .unwrap();
    let text = vec![
        format!("Name:              {}", contact.name),
        format!("Id:                {:?}", contact.id),
        format!("Last message time: {}", time.to_rfc3339()),
        format!("Description:       {}", contact.description),
    ]
    .join("\n");
    ("Contact info", text)
}

fn render_keybinds() -> (&'static str, String) {
    let normal_keybinds = KeyBinds::normal_default()
        .iter()
        .map(|(k, c)| format!("{} :{}", k, c.names()[0]))
        .collect::<Vec<_>>()
        .join("\n");
    let command_keybinds = KeyBinds::command_default()
        .iter()
        .map(|(k, c)| format!("{} :{}", k, c.names()[0]))
        .collect::<Vec<_>>()
        .join("\n");
    let compose_keybinds = KeyBinds::compose_default()
        .iter()
        .map(|(k, c)| format!("{} :{}", k, c.names()[0]))
        .collect::<Vec<_>>()
        .join("\n");
    let popup_keybinds = KeyBinds::popup_default()
        .iter()
        .map(|(k, c)| format!("{} :{}", k, c.names()[0]))
        .collect::<Vec<_>>()
        .join("\n");

    let text = format!(
        "Normal mode bindings\n{}\n\nCommand mode bindings\n{}\n\nCompose mode bindings\n{}\n\nPopup mode bindings\n{}",
        normal_keybinds, command_keybinds, compose_keybinds, popup_keybinds
    );

    ("Keybindings", text)
}

fn render_commands() -> (&'static str, String) {
    let mut commands = crate::commands::commands()
        .into_iter()
        .map(|c| {
            c.names()
                .into_iter()
                .map(|s| format!(":{s}"))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .collect::<Vec<_>>();
    commands.sort();
    let text = commands.join("\n");

    ("Commands", text)
}

fn render_command_line_history(tui_state: &TuiState) -> (&'static str, String) {
    let lines = tui_state
        .command_history
        .iter()
        .into_iter()
        .map(|c| format!(":{c}"))
        .collect::<Vec<_>>();

    ("Command history", lines.join("\n"))
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

fn render_scrollbar(frame: &mut Frame<'_>, area: Rect, length: usize, position: usize) {
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);

    let scrollable_distance = length.saturating_sub(area.height.into());

    let mut scrollbar_state = ScrollbarState::new(scrollable_distance).position(position);
    frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}
