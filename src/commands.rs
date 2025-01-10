use std::ffi::OsString;

use futures::channel::mpsc;

use crate::{
    message::BackendMessage,
    tui::{Mode, TuiState},
};

pub trait Command: std::fmt::Debug {
    fn execute(&self, tui_state: &mut TuiState, ba_tx: &mpsc::UnboundedSender<BackendMessage>);

    fn parse(args: pico_args::Arguments) -> Option<Self>
    where
        Self: Sized;
}

#[derive(Debug)]
pub struct Quit;

impl Command for Quit {
    fn execute(&self, _tui_state: &mut TuiState, _ba_tx: &mpsc::UnboundedSender<BackendMessage>) {
        panic!("quit")
    }

    fn parse(_args: pico_args::Arguments) -> Option<Self> {
        Some(Self)
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
            tui_state.message_list_state.select(None);
            ba_tx
                .unbounded_send(BackendMessage::LoadMessages(contact.thread_id.clone()))
                .unwrap();
        }
    }

    fn parse(_args: pico_args::Arguments) -> Option<Self> {
        Some(Self)
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
            tui_state.message_list_state.select(None);
            ba_tx
                .unbounded_send(BackendMessage::LoadMessages(contact.thread_id.clone()))
                .unwrap();
        }
    }

    fn parse(_args: pico_args::Arguments) -> Option<Self> {
        Some(Self)
    }
}

#[derive(Debug)]
pub struct NextMessage;

impl Command for NextMessage {
    fn execute(&self, tui_state: &mut TuiState, _ba_tx: &mpsc::UnboundedSender<BackendMessage>) {
        tui_state.message_list_state.select_next();
    }

    fn parse(_args: pico_args::Arguments) -> Option<Self> {
        Some(Self)
    }
}

#[derive(Debug)]
pub struct PrevMessage;

impl Command for PrevMessage {
    fn execute(&self, tui_state: &mut TuiState, _ba_tx: &mpsc::UnboundedSender<BackendMessage>) {
        tui_state.message_list_state.select_previous();
    }

    fn parse(_args: pico_args::Arguments) -> Option<Self> {
        Some(Self)
    }
}

#[derive(Debug)]
pub struct NormalMode;

impl Command for NormalMode {
    fn execute(&self, tui_state: &mut TuiState, _ba_tx: &mpsc::UnboundedSender<BackendMessage>) {
        tui_state.mode = Mode::Normal;
        tui_state.command.reset();
    }

    fn parse(_args: pico_args::Arguments) -> Option<Self> {
        Some(Self)
    }
}

#[derive(Debug)]
pub struct CommandMode;

impl Command for CommandMode {
    fn execute(&self, tui_state: &mut TuiState, _ba_tx: &mpsc::UnboundedSender<BackendMessage>) {
        tui_state.mode = Mode::Command;
    }

    fn parse(_args: pico_args::Arguments) -> Option<Self> {
        Some(Self)
    }
}

#[derive(Debug)]
pub struct ComposeMode;

impl Command for ComposeMode {
    fn execute(&self, tui_state: &mut TuiState, _ba_tx: &mpsc::UnboundedSender<BackendMessage>) {
        tui_state.mode = Mode::Compose;
    }

    fn parse(_args: pico_args::Arguments) -> Option<Self> {
        Some(Self)
    }
}

#[derive(Debug)]
pub struct SendMessage;

impl Command for SendMessage {
    fn execute(&self, tui_state: &mut TuiState, ba_tx: &mpsc::UnboundedSender<BackendMessage>) {
        let message_body = tui_state.compose.value().to_owned();
        tui_state.compose.reset();
        tui_state.mode = Mode::Normal;

        if let Some(contact) = tui_state
            .contact_list_state
            .selected()
            .and_then(|i| tui_state.contacts.get(i))
        {
            ba_tx
                .unbounded_send(BackendMessage::SendMessage(
                    contact.thread_id.clone(),
                    message_body,
                ))
                .unwrap();
        }
    }

    fn parse(_args: pico_args::Arguments) -> Option<Self> {
        Some(Self)
    }
}

#[derive(Debug)]
pub struct ExecuteCommand;

impl Command for ExecuteCommand {
    fn execute(&self, tui_state: &mut TuiState, ba_tx: &mpsc::UnboundedSender<BackendMessage>) {
        let value = tui_state.command.value().to_owned();
        tui_state.command.reset();
        tui_state.mode = Mode::Normal;

        let args = shell_words::split(&value)
            .unwrap()
            .into_iter()
            .map(|s| OsString::from(s))
            .collect();
        let mut pargs = pico_args::Arguments::from_vec(args);

        match pargs.subcommand().unwrap().unwrap().as_str() {
            "quit" => {
                Quit::parse(pargs).unwrap().execute(tui_state, ba_tx);
            }
            "next-contact" => {
                NextContact::parse(pargs).unwrap().execute(tui_state, ba_tx);
            }
            "prev-contact" => {
                PrevContact::parse(pargs).unwrap().execute(tui_state, ba_tx);
            }
            "next-message" => {
                NextMessage::parse(pargs).unwrap().execute(tui_state, ba_tx);
            }
            "prev-message" => {
                PrevMessage::parse(pargs).unwrap().execute(tui_state, ba_tx);
            }
            "mode-normal" => {
                NormalMode::parse(pargs).unwrap().execute(tui_state, ba_tx);
            }
            "mode-command" => {
                CommandMode::parse(pargs).unwrap().execute(tui_state, ba_tx);
            }
            "mode-compose" => {
                ComposeMode::parse(pargs).unwrap().execute(tui_state, ba_tx);
            }
            "send-message" => {
                SendMessage::parse(pargs).unwrap().execute(tui_state, ba_tx);
            }
            _ => {}
        }
    }

    fn parse(_args: pico_args::Arguments) -> Option<Self> {
        None
    }
}
