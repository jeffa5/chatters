use command_line::CommandLine;
use compose::Compose;
use contacts::Contacts;
use list::HorizontalList;
use list::VerticalList;
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
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Row;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::Table;
use ratatui::Frame;
use std::fmt::Display;
use std::path::PathBuf;
use textwrap::Options;

use crate::backends::Contact;
use crate::backends::ContactId;
use crate::config::Config;
use crate::keybinds::KeyBinds;
use crate::keybinds::KeyEvents;

mod command_line;
mod compose;
mod contacts;
mod list;
pub mod messages;
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
    pub app_name: String,
    pub self_id: Vec<u8>,
    pub contacts: Contacts,
    pub messages: Messages,
    pub compose: Compose,
    pub command_line: CommandLine,
    pub mode: Mode,
    pub popup: Option<Popup>,
    pub key_events: KeyEvents,
    pub config: Config,
    pub config_path: PathBuf,
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
            let age = if let Some(ts) = c.last_message_timestamp {
                biggest_duration_string(now.saturating_sub(ts))
            } else {
                String::new()
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
    let message_width = rect.width as usize - 1;
    let message_items = tui_state.messages.messages_by_ts.values().map(|m| {
        let sender_width = 20;
        let sender = tui_state
            .contacts
            .contact_by_id(&m.sender)
            .map(|c| c.name.clone())
            .unwrap();
        let sender = truncate_or_pad(sender, sender_width);
        let age = biggest_duration_string(
            now.saturating_sub(m.edits.last().map_or(m.timestamp, |e| e.timestamp)),
        );
        let sender_time = format!("{sender} {age:>3} ");

        let content_width = message_width
            .saturating_sub(sender_time.len())
            .saturating_sub(1);
        let content_indent = " ".repeat(sender_time.len());

        let content_lines = m.render(content_width);
        if content_lines.is_empty() {
            warn!(message:? = m; "Message with no information...");
        }

        let mut lines = Vec::new();
        for (i, line) in content_lines.into_iter().enumerate() {
            if i == 0 {
                lines.push(Line::from(vec![Span::from(sender_time.clone()), line]));
            } else {
                lines.push(Line::from(vec![Span::from(content_indent.clone()), line]));
            }
        }
        Text::from(lines)
    });
    let mut messages = VerticalList::new(message_items.collect());
    messages.set_selected_item_style(Style::new().reversed());

    let remaining_area = render_scrollbar(
        frame,
        rect,
        messages.len(),
        tui_state.messages.state.offset(),
    );

    frame.render_stateful_widget(&messages, remaining_area, &mut tui_state.messages.state);
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
    let revstyle = Style::new().reversed();

    frame.render_widget(
        Line::from(" ".repeat(rect.width as usize)).style(revstyle),
        rect,
    );

    let completions = tui_state
        .command_line
        .completions()
        .iter()
        .map(|c| Span::from(c.display.clone()).style(revstyle));

    let splits = Layout::horizontal([
        Constraint::Length(8),
        Constraint::Fill(1),
        Constraint::Length(4),
    ])
    .split(rect);

    frame.render_widget(
        Span::from(tui_state.mode.to_string()).style(revstyle),
        splits[0],
    );

    let mut completions_list = HorizontalList::new(completions.collect());
    completions_list.set_selected_item_style(Style::new().bold());
    frame.render_stateful_widget(
        &completions_list,
        splits[1],
        &mut tui_state.command_line.completions.list_state,
    );

    frame.render_widget(
        Span::from(tui_state.key_events.to_string()).style(revstyle),
        splits[2],
    );
}

fn render_command(frame: &mut Frame<'_>, rect: Rect, tui_state: &mut TuiState, _now: u64) {
    if tui_state.command_line.error.is_empty() {
        if matches!(tui_state.mode, Mode::Command { .. }) {
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
            let Some(message) = tui_state.messages.get_by_timestamp(*timestamp) else {
                warn!(timestamp:?; "No message with timestamp when rendering popup for message info");
                return;
            };
            render_message_info(width, tui_state, message)
        }
        PopupType::ContactInfo { id } => {
            let Some(contact) = tui_state
                .contacts
                .iter_contacts_and_groups()
                .find(|c| &c.id == id)
            else {
                warn!(id:?; "No contact with id when rendering popup for contact info");
                return;
            };
            render_contact_info(contact)
        }
        PopupType::Keybinds => render_keybinds(&tui_state.config.keybinds),
        PopupType::Commands => render_commands(),
        PopupType::CommandHistory => render_command_line_history(tui_state),
    };

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
) -> (&'static str, Text<'static>) {
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
    let mut text = vec![
        Line::from(format!("Sender name: {}", sender_name)),
        Line::from(format!("Sender id:   {}", hex::encode(&message.sender))),
        Line::from(format!("Time:        {}", time.to_rfc3339())),
        Line::from(""),
    ];
    let message_lines = message.render(width).into_iter().map(|s| Line::from(s));
    text.extend(message_lines);
    ("Message info", Text::from(text))
}

fn render_contact_info(contact: &Contact) -> (&'static str, Text) {
    let time = contact
        .last_message_timestamp
        .map(|ts| {
            let ts_seconds = ts / 1_000;
            let ts_nanos = (ts % 1_000) * 1_000_000;
            let time = chrono::DateTime::from_timestamp(
                ts_seconds.try_into().unwrap(),
                ts_nanos.try_into().unwrap(),
            )
            .unwrap();
            time.to_rfc3339()
        })
        .unwrap_or_else(|| "unknown".to_owned());
    let text = vec![
        Line::from(format!("Name:              {}", contact.name)),
        Line::from(format!("Id:                {}", contact.id)),
        Line::from(format!("Last message time: {}", time)),
        Line::from(format!("Description:       {}", contact.description)),
    ];
    ("Contact info", Text::from(text))
}

fn render_keybinds(keybindings: &KeyBinds) -> (&'static str, Text) {
    fn display_keybinds<'a>(bindings: impl Iterator<Item = (&'a KeyEvents, &'a String)>) -> String {
        let mut bs = bindings
            .map(|(k, c)| format!("{} = {}", k, c))
            .collect::<Vec<_>>();
        bs.sort();
        bs.join("\n")
    }
    let normal_keybinds = display_keybinds(keybindings.iter(Mode::Normal));
    let command_keybinds = display_keybinds(keybindings.iter(Mode::Command {
        previous: BasicMode::Normal,
    }));
    let compose_keybinds = display_keybinds(keybindings.iter(Mode::Compose));
    let popup_keybinds = display_keybinds(keybindings.iter(Mode::Popup));

    let text = format!(
        "Normal mode bindings\n{}\n\nCommand mode bindings\n{}\n\nCompose mode bindings\n{}\n\nPopup mode bindings\n{}",
        normal_keybinds, command_keybinds, compose_keybinds, popup_keybinds
    );

    ("Keybindings", Text::from(text))
}

fn render_commands() -> (&'static str, Text<'static>) {
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

    ("Commands", Text::from(text))
}

fn render_command_line_history(tui_state: &TuiState) -> (&'static str, Text<'static>) {
    let lines = tui_state
        .command_line
        .history
        .iter()
        .map(|c| format!(":{c}"))
        .collect::<Vec<_>>();

    ("Command history", Text::from(lines.join("\n")))
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
    let mut scrollbar_state = ScrollbarState::new(scrollable_distance).position(position);
    frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);

    Rect {
        width: area.width.saturating_sub(1),
        ..area
    }
}
