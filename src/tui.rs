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
use ratatui::Frame;

use crate::backends::Contact;
use crate::backends::Message;

#[derive(Debug)]
pub struct TuiState {
    pub contact_list_state: ListState,
    pub message_list_state: ListState,
    pub contacts: Vec<Contact>,
    pub contacts_by_id: BTreeMap<Uuid, Contact>,
    pub messages: Vec<Message>,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            contact_list_state: ListState::default(),
            message_list_state: ListState::default(),
            contacts: Vec::new(),
            contacts_by_id: BTreeMap::new(),
            messages: Vec::new(),
        }
    }
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

    let contact_items = tui_state
        .contacts
        .iter()
        .map(|c| format!("{}", c.name));
    let contacts = List::new(contact_items)
        .highlight_style(Style::new().reversed())
        .block(b.clone().title("Contacts"));

    let message_items = tui_state.messages.iter().map(|m| {
        format!(
            "{:?}: {}",
            tui_state
                .contacts_by_id
                .get(&m.sender)
                .map(|c| c.name.as_str())
                .unwrap_or(&m.sender.to_string()),
            m.content
        )
    });
    let messages = List::new(message_items)
        .highlight_style(Style::new().reversed())
        .block(b.clone().title("Messages"));
    frame.render_stateful_widget(contacts, main_rect[0], &mut tui_state.contact_list_state);
    frame.render_stateful_widget(messages, main_rect[1], &mut tui_state.message_list_state);

    let status_line =
        Paragraph::new(Line::from("mode:Normal")).block(b.clone().title("Status line"));
    frame.render_widget(status_line, chunks[1]);

    let exline = Paragraph::new(Line::from(":")).block(b.title("Exline"));
    frame.render_widget(exline, chunks[2]);
}
