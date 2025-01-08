use std::collections::BTreeMap;

use presage::libsignal_service::prelude::Uuid;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::List;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Row;
use ratatui::widgets::Table;
use ratatui::widgets::TableState;
use ratatui::Frame;
use tui_input::Input;

use crate::backends::Contact;
use crate::backends::Message;

#[derive(Debug, Default)]
pub enum Mode {
    #[default]
    Normal,
    Command,
    Compose,
}

#[derive(Debug, Default)]
pub struct TuiState {
    pub contact_list_state: ListState,
    pub message_list_state: TableState,
    pub contacts: Vec<Contact>,
    pub contacts_by_id: BTreeMap<Uuid, Contact>,
    pub messages: Vec<Message>,
    pub compose: Input,
    pub command: Input,
    pub mode: Mode,
}

pub fn render(frame: &mut Frame<'_>, tui_state: &mut TuiState) {
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

    let contact_items = tui_state.contacts.iter().map(|c| format!("{}", c.name));
    let contacts = List::new(contact_items)
        .highlight_style(Style::new().reversed())
        .block(b.clone().title("Contacts"));
    frame.render_stateful_widget(contacts, main_rect[0], &mut tui_state.contact_list_state);

    let message_items = tui_state.messages.iter().map(|m| {
        Row::new(vec![
            tui_state
                .contacts_by_id
                .get(&m.sender)
                .map(|c| c.name.clone())
                .unwrap_or(m.sender.to_string()),
            m.content.to_string(),
        ])
        .height(m.content.lines().count() as u16)
    });
    let messages = Table::new(message_items, [Constraint::Fill(1), Constraint::Fill(4)])
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
