use futures::channel::mpsc;

use crate::{message::BackendMessage, tui::TuiState};

pub trait Command: std::fmt::Debug {
    fn execute(&self, tui_state: &mut TuiState, ba_tx: &mpsc::UnboundedSender<BackendMessage>);
}

#[derive(Debug)]
pub struct Quit;

impl Command for Quit {
    fn execute(&self, _tui_state: &mut TuiState, _ba_tx: &mpsc::UnboundedSender<BackendMessage>) {
        panic!("quit")
    }
}

#[derive(Debug)]
pub struct NextContact;

impl Command for NextContact {
    fn execute(&self, tui_state: &mut TuiState, ba_tx: &mpsc::UnboundedSender<BackendMessage>) {
        tui_state.contact_list_state.select_next();
        if let Some(contact) = tui_state
            .contact_list_state
            .selected()
            .and_then(|i| tui_state.contacts.get(i))
        {
            tui_state.messages.clear();
            ba_tx
                .unbounded_send(BackendMessage::LoadMessages(contact.thread_id.clone()))
                .unwrap();
        }
    }
}

#[derive(Debug)]
pub struct PrevContact;

impl Command for PrevContact {
    fn execute(&self, tui_state: &mut TuiState, ba_tx: &mpsc::UnboundedSender<BackendMessage>) {
        tui_state.contact_list_state.select_previous();
        if let Some(contact) = tui_state
            .contact_list_state
            .selected()
            .and_then(|i| tui_state.contacts.get(i))
        {
            tui_state.messages.clear();
            ba_tx
                .unbounded_send(BackendMessage::LoadMessages(contact.thread_id.clone()))
                .unwrap();
        }
    }
}

#[derive(Debug)]
pub struct NextMessage;

impl Command for NextMessage {
    fn execute(&self, tui_state: &mut TuiState, _ba_tx: &mpsc::UnboundedSender<BackendMessage>) {
        tui_state.message_list_state.select_next();
    }
}

#[derive(Debug)]
pub struct PrevMessage;

impl Command for PrevMessage {
    fn execute(&self, tui_state: &mut TuiState, _ba_tx: &mpsc::UnboundedSender<BackendMessage>) {
        tui_state.message_list_state.select_previous();
    }
}
