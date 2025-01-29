use std::{
    ffi::OsString,
    io::{Read, Seek, Write as _},
    path::PathBuf,
};

use futures::channel::mpsc;
use log::{debug, warn};
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
    #[error("Missing argument {0}")]
    MissingArgument(String),
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

    fn complete(&self) -> Vec<String>;
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
    v.push(Box::new(ClearCompose::default()));
    v.push(Box::new(DownloadAttachments::default()));
    v.push(Box::new(OpenAttachments::default()));
    v.push(Box::new(ExecuteCommand::default()));
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
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
        let index = args
            .free_from_str()
            .map_err(|_e| Error::MissingArgument("index".to_owned()))?;
        *self = Self { index };
        Ok(())
    }

    fn default() -> Self {
        Self { index: 0 }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["select-message"]
    }

    fn complete(&self) -> Vec<String> {
        Vec::new()
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
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
        let message_body = tui_state.compose.lines().join("\n").trim().to_owned();
        tui_state.compose = TextArea::default();
        NormalMode.execute(tui_state, ba_tx).unwrap();

        if message_body.is_empty() {
            return Ok(CommandSuccess::Nothing);
        }

        let attachments = Vec::new();

        if let Some(contact) = tui_state
            .contact_list_state
            .selected()
            .and_then(|i| tui_state.contacts.get(i))
        {
            ba_tx
                .unbounded_send(BackendMessage::SendMessage(
                    contact.thread_id.clone(),
                    MessageContent::Text(message_body, attachments),
                    // TODO: enable sending replies to messages
                    None,
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
    }
}

#[derive(Debug)]
pub struct React {
    emoji: String,
}

impl Command for React {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let Some(e) = emojis::get_by_shortcode(&self.emoji) else {
            return Err(Error::InvalidArgument {
                arg: "emoji".to_owned(),
                value: self.emoji.clone(),
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
                None,
            ))
            .unwrap();
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        let emoji = args
            .free_from_str()
            .map_err(|_e| Error::MissingArgument("emoji".to_owned()))?;
        *self = Self { emoji };
        Ok(())
    }

    fn default() -> Self {
        Self {
            emoji: String::new(),
        }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["react"]
    }

    fn complete(&self) -> Vec<String> {
        if self.emoji.is_empty() {
            return Vec::new();
        }
        emojis::iter()
            .flat_map(|e| e.shortcodes())
            .filter(|s| s.starts_with(&self.emoji))
            .map(|s| s.to_owned())
            .take(10)
            .collect()
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
                None,
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
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

        debug!(pargs:? = pargs; "Parsed arguments for command");
        let subcmd = loop {
            let Some(subcmd) = pargs.subcommand().unwrap() else {
                return Ok(CommandSuccess::Nothing);
            };
            if self.names().contains(&subcmd.as_str()) {
                continue;
            } else {
                break subcmd;
            }
        };
        let commands = commands();
        let command = commands
            .into_iter()
            .find(|c| c.names().contains(&subcmd.as_str()));

        if let Some(mut command) = command {
            if let Err(error) = command.parse(pargs) {
                tui_state.command_error = error.to_string();
                return Ok(CommandSuccess::Nothing);
            }
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
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

    fn complete(&self) -> Vec<String> {
        Vec::new()
    }
}

#[derive(Debug)]
pub struct ClearCompose;

impl Command for ClearCompose {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.compose = TextArea::default();
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
        vec!["clear-compose"]
    }

    fn complete(&self) -> Vec<String> {
        Vec::new()
    }
}

#[derive(Debug)]
pub struct DownloadAttachments {
    // TODO: change to vec of indices
    index: Option<usize>,
}

impl Command for DownloadAttachments {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        if let Some(message) = tui_state.selected_message() {
            if let Some(index) = self.index {
                if let Some(attachment) = message.attachments.get(index) {
                    ba_tx
                        .unbounded_send(BackendMessage::DownloadAttachment(
                            message.thread.clone(),
                            message.timestamp,
                            attachment.handle,
                        ))
                        .unwrap();
                }
            } else {
                for attachment in &message.attachments {
                    ba_tx
                        .unbounded_send(BackendMessage::DownloadAttachment(
                            message.thread.clone(),
                            message.timestamp,
                            attachment.handle,
                        ))
                        .unwrap();
                }
            }
        }
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        let index = args.opt_free_from_str().unwrap();
        *self = Self { index };
        Ok(())
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        Self { index: None }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["download-attachments"]
    }

    fn complete(&self) -> Vec<String> {
        Vec::new()
    }
}

#[derive(Debug)]
pub struct OpenAttachments {
    // TODO: change to vec of indices
    index: Option<usize>,
}

impl Command for OpenAttachments {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let Some(message) = tui_state.selected_message() else {
            return Err(Error::NoMessageSelected);
        };
        let open_attachment = |path: &Option<PathBuf>| {
            if let Some(path) = path {
                debug!(path:? = path; "Opening attachment");
                open::that_detached(path).unwrap();
            }
        };
        if let Some(index) = self.index {
            if let Some(attachment) = message.attachments.get(index) {
                open_attachment(&attachment.downloaded_file_path);
            }
        } else {
            for attachment in &message.attachments {
                open_attachment(&attachment.downloaded_file_path);
            }
        }
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        let index = args.opt_free_from_str().unwrap();
        *self = Self { index };
        Ok(())
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        Self { index: None }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["open-attachments"]
    }

    fn complete(&self) -> Vec<String> {
        // TODO: get tui_state here and present the indices of attachments
        Vec::new()
    }
}
