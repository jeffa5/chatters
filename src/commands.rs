use std::{
    ffi::OsString,
    io::{Read, Seek, Write as _},
};

use futures::channel::mpsc;
use log::warn;
use tui_textarea::TextArea;

use crate::{
    backends::MessageContent,
    message::BackendMessage,
    tui::{Mode, TuiState},
};

pub enum CommandSuccess {
    Nothing,
    Quit,
    Clear,
}

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
    ) -> Result<CommandSuccess>;

    fn parse(&mut self, args: pico_args::Arguments) -> Result<()>;

    fn default() -> Self
    where
        Self: Sized;

    fn names(&self) -> Vec<&'static str>;
}

pub fn commands() -> Vec<Box<dyn Command>> {
    let mut v: Vec<Box<dyn Command>> = Vec::new();
    v.push(Box::new(Quit::default()));
    v.push(Box::new(NextContact::default()));
    v.push(Box::new(PrevContact::default()));
    v.push(Box::new(NextMessage::default()));
    v.push(Box::new(PrevMessage::default()));
    v.push(Box::new(SelectMessage::default()));
    v.push(Box::new(NormalMode::default()));
    v.push(Box::new(CommandMode::default()));
    v.push(Box::new(ComposeMode::default()));
    v.push(Box::new(SendMessage::default()));
    v.push(Box::new(React::default()));
    v.push(Box::new(Unreact::default()));
    v.push(Box::new(ReloadContacts::default()));
    v.push(Box::new(ReloadMessages::default()));
    v.push(Box::new(ComposeInEditor::default()));
    v
}

#[derive(Debug)]
pub struct Quit;

impl Command for Quit {
    fn execute(
        &self,
        _tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        Ok(CommandSuccess::Quit)
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["quit"]
    }
}

#[derive(Debug)]
pub struct NextContact;

impl Command for NextContact {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.contact_list_state.select_next();
        if let Some(contact) = tui_state
            .contact_list_state
            .selected()
            .and_then(|i| tui_state.contacts.get(i))
        {
            tui_state.messages.clear();
            tui_state.message_list_state.select(None);
            ba_tx
                .unbounded_send(BackendMessage::LoadMessages {
                    thread: contact.thread_id.clone(),
                    start_ts: std::ops::Bound::Unbounded,
                    end_ts: std::ops::Bound::Unbounded,
                })
                .unwrap();
        }
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["next-contact"]
    }
}

#[derive(Debug)]
pub struct PrevContact;

impl Command for PrevContact {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.contact_list_state.select_previous();
        if let Some(contact) = tui_state
            .contact_list_state
            .selected()
            .and_then(|i| tui_state.contacts.get(i))
        {
            tui_state.messages.clear();
            tui_state.message_list_state.select(None);
            ba_tx
                .unbounded_send(BackendMessage::LoadMessages {
                    thread: contact.thread_id.clone(),
                    start_ts: std::ops::Bound::Unbounded,
                    end_ts: std::ops::Bound::Unbounded,
                })
                .unwrap();
        }
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["prev-contact"]
    }
}

#[derive(Debug)]
pub struct NextMessage;

impl Command for NextMessage {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.message_list_state.select_next();
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["next-message"]
    }
}

#[derive(Debug)]
pub struct PrevMessage;

impl Command for PrevMessage {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.message_list_state.select_previous();
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["prev-message"]
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
    ) -> Result<CommandSuccess> {
        let abs_index: usize = self.index.abs().try_into().unwrap();
        if self.index < 0 {
            let num_messages = tui_state.messages.len();
            tui_state
                .message_list_state
                .select(Some(num_messages - (abs_index % num_messages)));
        } else {
            tui_state.message_list_state.select(Some(abs_index));
        }
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        let index = args.free_from_str().map_err(|_e| Error::InvalidArgument {
            arg: "index".to_owned(),
            value: "".to_owned(),
        })?;
        *self = Self { index };
        Ok(())
    }

    fn default() -> Self {
        Self { index: 0 }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["select-message"]
    }
}

#[derive(Debug)]
pub struct NormalMode;

impl Command for NormalMode {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.mode = Mode::Normal;
        tui_state.command = TextArea::default();
        tui_state.command_completions.clear();
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["mode-normal"]
    }
}

#[derive(Debug)]
pub struct CommandMode;

impl Command for CommandMode {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.mode = Mode::Command;
        tui_state.command_error.clear();
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["mode-command"]
    }
}

#[derive(Debug)]
pub struct ComposeMode;

impl Command for ComposeMode {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.mode = Mode::Compose;
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["mode-compose"]
    }
}

#[derive(Debug)]
pub struct SendMessage;

impl Command for SendMessage {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let message_body = tui_state.compose.lines().join("\n");
        tui_state.compose = TextArea::default();
        NormalMode.execute(tui_state, ba_tx).unwrap();

        if message_body.is_empty() {
            return Ok(CommandSuccess::Nothing);
        }

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
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["send-message"]
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
    ) -> Result<CommandSuccess> {
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
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        let reaction = args.free_from_str().unwrap();
        *self = Self { reaction };
        Ok(())
    }

    fn default() -> Self {
        Self {
            reaction: String::new(),
        }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["react"]
    }
}

#[derive(Debug)]
pub struct Unreact;

impl Command for Unreact {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
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
            return Ok(CommandSuccess::Nothing);
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
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["unreact"]
    }
}

#[derive(Debug)]
pub struct ExecuteCommand;

impl Command for ExecuteCommand {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let value = tui_state.command.lines().join("\n");
        tui_state.command = TextArea::default();
        NormalMode.execute(tui_state, ba_tx).unwrap();

        let args = shell_words::split(&value)
            .unwrap()
            .into_iter()
            .map(|s| OsString::from(s))
            .collect();
        let mut pargs = pico_args::Arguments::from_vec(args);

        let subcmd = pargs.subcommand().unwrap().unwrap();
        let commands = commands();
        let command = commands
            .into_iter()
            .find(|c| c.names().contains(&subcmd.as_str()));

        if let Some(mut command) = command {
            command.parse(pargs).unwrap();
            let ret = command.execute(tui_state, ba_tx)?;
            Ok(ret)
        } else {
            Err(Error::UnknownCommand(subcmd.to_owned()))
        }
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["execute-command"]
    }
}

#[derive(Debug)]
pub struct ReloadContacts;

impl Command for ReloadContacts {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.contacts.clear();
        tui_state.contacts_by_id.clear();
        tui_state.contact_list_state.select(None);
        ba_tx.unbounded_send(BackendMessage::LoadContacts).unwrap();
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["reload-contacts"]
    }
}

#[derive(Debug)]
pub struct ReloadMessages;

impl Command for ReloadMessages {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.messages.clear();
        tui_state.message_list_state.select(None);
        if let Some(contact) = tui_state
            .contact_list_state
            .selected()
            .and_then(|i| tui_state.contacts.get(i))
        {
            ba_tx
                .unbounded_send(BackendMessage::LoadMessages {
                    thread: contact.thread_id.clone(),
                    start_ts: std::ops::Bound::Unbounded,
                    end_ts: std::ops::Bound::Unbounded,
                })
                .unwrap();
        }
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["reload-messages"]
    }
}

#[derive(Debug)]
pub struct ComposeInEditor;

impl Command for ComposeInEditor {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let compose_content = tui_state.compose.lines().join("\n");
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        tmpfile.write_all(compose_content.as_bytes()).unwrap();
        let editor = std::env::var("EDITOR").unwrap_or("vim".to_owned());
        let status = std::process::Command::new(editor)
            .arg(tmpfile.path())
            .status()
            .unwrap();
        if status.success() {
            let mut compose_content = String::new();
            tmpfile.seek(std::io::SeekFrom::Start(0)).unwrap();
            tmpfile.read_to_string(&mut compose_content).unwrap();
            let compose_lines = compose_content.lines().map(|l| l.to_owned()).collect();
            (*tui_state).compose = TextArea::new(compose_lines);
        } else {
            warn!("Not using compose content from external editor due to error status");
        }
        Ok(CommandSuccess::Clear)
    }

    fn parse(&mut self, _args: pico_args::Arguments) -> Result<()> {
        Ok(())
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["compose-in-editor"]
    }
}
