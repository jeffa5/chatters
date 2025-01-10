use std::ffi::OsString;

use futures::channel::mpsc;

use crate::{
    backends::MessageContent,
    message::BackendMessage,
    tui::{Mode, TuiState},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid argument {arg:?} with value {value:?}")]
    InvalidArgument { arg: String, value: String },
    #[error("No contact selected")]
    NoContactSelected,
    #[error("No message selected")]
    NoMessageSelected,
    #[error("Unknown command {0:?}")]
    UnknownCommand(String),
}

type Result<T> = std::result::Result<T, Error>;

pub trait Command: std::fmt::Debug {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()>;

    fn parse(args: pico_args::Arguments) -> Result<Self>
    where
        Self: Sized;
}

#[derive(Debug)]
pub struct Quit;

impl Command for Quit {
    fn execute(
        &self,
        _tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()> {
        panic!("quit")
    }

    fn parse(_args: pico_args::Arguments) -> Result<Self> {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct NextContact;

impl Command for NextContact {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()> {
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
        Ok(())
    }

    fn parse(_args: pico_args::Arguments) -> Result<Self> {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct PrevContact;

impl Command for PrevContact {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()> {
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
        Ok(())
    }

    fn parse(_args: pico_args::Arguments) -> Result<Self> {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct NextMessage;

impl Command for NextMessage {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()> {
        tui_state.message_list_state.select_next();
        Ok(())
    }

    fn parse(_args: pico_args::Arguments) -> Result<Self> {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct PrevMessage;

impl Command for PrevMessage {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()> {
        tui_state.message_list_state.select_previous();
        Ok(())
    }

    fn parse(_args: pico_args::Arguments) -> Result<Self> {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct SelectMessage {
    pub index: isize,
}

impl Command for SelectMessage {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()> {
        let abs_index: usize = self.index.abs().try_into().unwrap();
        if self.index < 0 {
            let num_messages = tui_state.messages.len();
            tui_state
                .message_list_state
                .select(Some(num_messages - (abs_index % num_messages)));
        } else {
            tui_state.message_list_state.select(Some(abs_index));
        }
        Ok(())
    }

    fn parse(mut args: pico_args::Arguments) -> Result<Self> {
        let index = args.free_from_str().map_err(|_e| Error::InvalidArgument {
            arg: "index".to_owned(),
            value: "".to_owned(),
        })?;
        Ok(Self { index })
    }
}

#[derive(Debug)]
pub struct NormalMode;

impl Command for NormalMode {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()> {
        tui_state.mode = Mode::Normal;
        tui_state.command.reset();
        Ok(())
    }

    fn parse(_args: pico_args::Arguments) -> Result<Self> {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct CommandMode;

impl Command for CommandMode {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()> {
        tui_state.mode = Mode::Command;
        tui_state.command_error.clear();
        Ok(())
    }

    fn parse(_args: pico_args::Arguments) -> Result<Self> {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct ComposeMode;

impl Command for ComposeMode {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()> {
        tui_state.mode = Mode::Compose;
        Ok(())
    }

    fn parse(_args: pico_args::Arguments) -> Result<Self> {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct SendMessage;

impl Command for SendMessage {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()> {
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
                    MessageContent::Text(message_body),
                ))
                .unwrap();
        }
        Ok(())
    }

    fn parse(_args: pico_args::Arguments) -> Result<Self> {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct React {
    reaction: String,
}

impl Command for React {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()> {
        let Some(e) = emojis::get_by_shortcode(&self.reaction) else {
            return Err(Error::InvalidArgument {
                arg: "reaction".to_owned(),
                value: self.reaction.clone(),
            });
        };

        let Some(contact) = tui_state
            .contact_list_state
            .selected()
            .and_then(|i| tui_state.contacts.get(i))
        else {
            return Err(Error::NoContactSelected);
        };

        let Some(selected_message) = tui_state
            .message_list_state
            .selected()
            .and_then(|i| tui_state.messages.get_by_index(i))
        else {
            return Err(Error::NoMessageSelected);
        };

        ba_tx
            .unbounded_send(BackendMessage::SendMessage(
                contact.thread_id.clone(),
                MessageContent::Reaction(
                    selected_message.sender,
                    selected_message.timestamp,
                    e.as_str().to_owned(),
                    false,
                ),
            ))
            .unwrap();
        Ok(())
    }

    fn parse(mut args: pico_args::Arguments) -> Result<Self>
    where
        Self: Sized,
    {
        let reaction = args.free_from_str().unwrap();
        Ok(Self { reaction })
    }
}

#[derive(Debug)]
pub struct Unreact;

impl Command for Unreact {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()> {
        let Some(contact) = tui_state
            .contact_list_state
            .selected()
            .and_then(|i| tui_state.contacts.get(i))
        else {
            return Err(Error::NoContactSelected);
        };

        let Some(selected_message) = tui_state
            .message_list_state
            .selected()
            .and_then(|i| tui_state.messages.get_by_index(i))
        else {
            return Err(Error::NoMessageSelected);
        };

        let Some(reaction) = selected_message
            .reactions
            .iter()
            .find(|r| r.author == tui_state.self_uuid)
            .map(|r| r.emoji.clone())
        else {
            return Ok(());
        };

        ba_tx
            .unbounded_send(BackendMessage::SendMessage(
                contact.thread_id.clone(),
                MessageContent::Reaction(
                    selected_message.sender,
                    selected_message.timestamp,
                    reaction,
                    true,
                ),
            ))
            .unwrap();
        Ok(())
    }

    fn parse(_args: pico_args::Arguments) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct ExecuteCommand;

impl Command for ExecuteCommand {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<()> {
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
                Quit::parse(pargs).unwrap().execute(tui_state, ba_tx)?;
            }
            "next-contact" => {
                NextContact::parse(pargs)
                    .unwrap()
                    .execute(tui_state, ba_tx)?;
            }
            "prev-contact" => {
                PrevContact::parse(pargs)
                    .unwrap()
                    .execute(tui_state, ba_tx)?;
            }
            "next-message" => {
                NextMessage::parse(pargs)
                    .unwrap()
                    .execute(tui_state, ba_tx)?;
            }
            "prev-message" => {
                PrevMessage::parse(pargs)
                    .unwrap()
                    .execute(tui_state, ba_tx)?;
            }
            "select-message" => {
                SelectMessage::parse(pargs)
                    .unwrap()
                    .execute(tui_state, ba_tx)?;
            }
            "mode-normal" => {
                NormalMode::parse(pargs)
                    .unwrap()
                    .execute(tui_state, ba_tx)?;
            }
            "mode-command" => {
                CommandMode::parse(pargs)
                    .unwrap()
                    .execute(tui_state, ba_tx)?;
            }
            "mode-compose" => {
                ComposeMode::parse(pargs)
                    .unwrap()
                    .execute(tui_state, ba_tx)?;
            }
            "send-message" => {
                SendMessage::parse(pargs)
                    .unwrap()
                    .execute(tui_state, ba_tx)?;
            }
            "react" => {
                React::parse(pargs).unwrap().execute(tui_state, ba_tx)?;
            }
            "unreact" => {
                Unreact::parse(pargs).unwrap().execute(tui_state, ba_tx)?;
            }
            subcmd => return Err(Error::UnknownCommand(subcmd.to_owned())),
        }
        Ok(())
    }

    fn parse(_args: pico_args::Arguments) -> Result<Self> {
        Ok(Self)
    }
}
