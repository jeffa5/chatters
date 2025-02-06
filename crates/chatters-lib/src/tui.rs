use command_line::CommandLine;
use compose::Compose;
use contacts::Contacts;
use log::warn;
use messages::Message;
use messages::Messages;
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
use ratatui::Frame;
use std::fmt::Display;
use textwrap::Options;

use crate::backends::Contact;
use crate::backends::ContactId;
use crate::keybinds::KeyBinds;

mod command_line;
mod compose;
mod contacts;
mod messages;
pub use messages::Quote;

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

#[derive(Debug, Clone, Copy)]
pub enum BasicMode {
    Normal,
    Popup,
    Compose,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum Mode {
    #[default]
    Normal,
    Command {
        previous: BasicMode,
    },
    Compose,
    Popup,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Mode::Normal => "Normal",
            Mode::Command { previous: _ } => "Command",
            Mode::Compose => "Compose",
            Mode::Popup => "Popup",
        };
        f.write_str(s)
    }
}

#[derive(Debug)]
pub struct Popup {
    pub typ: PopupType,
    pub scroll: u16,
}

impl Popup {
    pub fn new(typ: PopupType) -> Self {
        Self { typ, scroll: 0 }
    }
}

#[derive(Debug)]
pub enum PopupType {
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
    pub message_list_state: ListState,
    pub contacts: Contacts,
    pub messages: Messages,
    pub compose: Compose,
    pub command_line: CommandLine,
    pub mode: Mode,
    pub popup: Option<Popup>,
}

impl TuiState {
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
    let block = Block::new().borders(Borders::RIGHT);
    let area = block.inner(rect);
    frame.render_widget(block, rect);

    let contacts = Table::new(contact_items, [Constraint::Fill(1), Constraint::Length(3)])
        .row_highlight_style(Style::new().reversed());

    let remaining_area = render_scrollbar(
        frame,
        area,
        contact_items_len,
        tui_state.contacts.state.offset(),
    );

    frame.render_stateful_widget(contacts, remaining_area, &mut tui_state.contacts.state);
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

    let remaining_area = render_scrollbar(
        frame,
        rect,
        messages.len(),
        tui_state.message_list_state.offset(),
    );

    frame.render_stateful_widget(&messages, remaining_area, &mut tui_state.message_list_state);
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
    let completions = tui_state.command_line.completions().join(" ");
    let status_line = Paragraph::new(Line::from(format!("{} {}", tui_state.mode, completions)))
        .style(Style::new().reversed());
    frame.render_widget(status_line, rect);
}

fn render_command(frame: &mut Frame<'_>, rect: Rect, tui_state: &mut TuiState, _now: u64) {
    if tui_state.command_line.error.is_empty() {
        if matches!(tui_state.mode, Mode::Command { .. }) {
            let value = tui_state.command_line.text();
            (format!(":{}", value), Style::new());
            frame.render_widget(Line::from(":"), rect);
            let inner_rect = rect.inner(Margin {
                horizontal: 1,
                vertical: 0,
            });
            tui_state
                .command_line
                .textarea()
                .set_cursor_line_style(Style::new());
            frame.render_widget(&*tui_state.command_line.textarea(), inner_rect);
        } else {
            frame.render_widget(ratatui::widgets::Clear, rect);
        }
    } else {
        frame.render_widget(
            Paragraph::new(tui_state.command_line.error.clone()).set_style(Style::new().red()),
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
    let (title, text) = match &popup.typ {
        PopupType::MessageInfo { timestamp } => {
            let message = tui_state.messages.get_by_timestamp(*timestamp).unwrap();
            render_message_info(width, tui_state, message)
        }
        PopupType::ContactInfo { id } => {
            let contact = tui_state
                .contacts
                .iter_contacts_and_groups()
                .find(|c| &c.id == id)
                .unwrap();
            render_contact_info(contact)
        }
        PopupType::Keybinds => render_keybinds(),
        PopupType::Commands => render_commands(),
        PopupType::CommandHistory => render_command_line_history(tui_state),
    };

    let text = wrap_text(&text, width);

    let line_count = text.lines.len() as u16;
    let max_scroll = line_count.saturating_sub(area.height.saturating_sub(2));
    let popup = tui_state.popup.as_mut().unwrap();
    popup.scroll = popup.scroll.min(max_scroll);
    let block = Block::bordered().title(title);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let remaining_area =
        render_scrollbar(frame, inner_area, line_count.into(), popup.scroll.into());

    let para = Paragraph::new(text).scroll((popup.scroll, 0));
    frame.render_widget(para, remaining_area);
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
    let keybindings = KeyBinds::default();
    let normal_keybinds = keybindings
        .iter(Mode::Normal)
        .map(|(k, c)| format!("{} :{}", k, c.names()[0]))
        .collect::<Vec<_>>()
        .join("\n");
    let command_keybinds = keybindings
        .iter(Mode::Command {
            previous: BasicMode::Normal,
        })
        .map(|(k, c)| format!("{} :{}", k, c.names()[0]))
        .collect::<Vec<_>>()
        .join("\n");
    let compose_keybinds = keybindings
        .iter(Mode::Compose)
        .map(|(k, c)| format!("{} :{}", k, c.names()[0]))
        .collect::<Vec<_>>()
        .join("\n");
    let popup_keybinds = keybindings
        .iter(Mode::Popup)
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
        .command_line
        .history
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

fn render_scrollbar(frame: &mut Frame<'_>, area: Rect, length: usize, position: usize) -> Rect {
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);

    let scrollable_distance = length.saturating_sub(area.height.into());
    if scrollable_distance == 0 {
        return area;
    }

    let mut scrollbar_state = ScrollbarState::new(scrollable_distance).position(position);
    frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);

    Rect {
        width: area.width.saturating_sub(1),
        ..area
    }
}
