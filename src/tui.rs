use std::collections::BTreeMap;

use presage::libsignal_service::prelude::Uuid;
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
use ratatui::widgets::Paragraph;
use ratatui::widgets::Row;
use ratatui::widgets::Table;
use ratatui::widgets::TableState;
use ratatui::Frame;
use tui_input::Input;

use crate::backends::Contact;
use crate::backends::Message;

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
pub struct TuiState {
    pub contact_list_state: TableState,
    pub message_list_state: TableState,
    pub contacts: Vec<Contact>,
    pub contacts_by_id: BTreeMap<Uuid, Contact>,
    pub messages: BTreeMap<u64, Message>,
    pub compose: Input,
    pub command: Input,
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
            biggest_duration_string(now - c.last_message_timestamp)
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

    let message_items = tui_state.messages.values().map(|m| {
        let sender = tui_state
            .contacts_by_id
            .get(&m.sender)
            .map(|c| c.name.clone())
            .unwrap_or(m.sender.to_string());
        let age = biggest_duration_string(now - m.timestamp);
        Row::new(vec![
            Text::from(sender),
            Text::from(age).alignment(Alignment::Right),
            Text::from(m.content.clone()),
        ])
        .height(m.content.lines().count() as u16)
    });
    let messages = Table::new(
        message_items,
        [
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(4),
        ],
    )
    .row_highlight_style(Style::new().reversed())
    .block(b.clone().title("Messages"));

    let message_rect = Layout::vertical([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(main_rect[1]);

    frame.render_stateful_widget(messages, message_rect[0], &mut tui_state.message_list_state);

    let compose = Paragraph::new(tui_state.compose.value()).block(b.clone().title("Compose"));
    frame.render_widget(compose, message_rect[1]);
    if matches!(tui_state.mode, Mode::Compose) {
        frame.set_cursor_position((
            // Put cursor past the end of the input text
            message_rect[1].x + tui_state.compose.visual_cursor() as u16 + 1,
            // Move one line down, from the border to the input line
            message_rect[1].y + 1,
        ))
    }

    let status_line = Paragraph::new(Line::from(format!("mode:{:?}", tui_state.mode)))
        .block(b.clone().title("Status line"));
    frame.render_widget(status_line, chunks[1]);

    let exline = Paragraph::new(tui_state.command.value()).block(b.title("Exline"));
    frame.render_widget(exline, chunks[2]);
    if matches!(tui_state.mode, Mode::Command) {
        frame.set_cursor_position((
            // Put cursor past the end of the input text
            chunks[2].x + tui_state.command.visual_cursor() as u16 + 1,
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
