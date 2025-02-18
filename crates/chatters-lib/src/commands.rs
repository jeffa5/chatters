use std::{
    convert::Infallible,
    env::current_dir,
    ffi::OsString,
    fs::read_dir,
    io::{Read, Seek, Write as _},
    path::PathBuf,
    process::Stdio,
    str::FromStr,
    sync::LazyLock,
};

use futures::channel::mpsc;
use log::{debug, warn};

use crate::{
    backends::MessageContent,
    message::BackendMessage,
    tui::{Mode, Popup, PopupType, Quote, TuiState},
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
    #[error("Unknown arguments to command: {0}")]
    UnknownArguments(String),
    #[error("{0}")]
    Failure(String),
}

type Result<T> = std::result::Result<T, Error>;

pub trait Command: std::fmt::Debug {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess>;

    fn parse(&mut self, args: pico_args::Arguments) -> Result<()> {
        check_unused_args(args)?;
        Ok(())
    }

    fn default() -> Self
    where
        Self: Sized;

    fn names(&self) -> Vec<&'static str>;

    fn complete(&self, _tui_state: &TuiState, _args: &str) -> Vec<Completion> {
        Vec::new()
    }

    fn dyn_clone(&self) -> Box<dyn Command>;
}

pub fn commands() -> Vec<Box<dyn Command>> {
    let mut v: Vec<Box<dyn Command>> = Vec::new();
    v.push(Box::new(Quit::default()));
    v.push(Box::new(NextContact::default()));
    v.push(Box::new(PrevContact::default()));
    v.push(Box::new(NextMessage::default()));
    v.push(Box::new(PrevMessage::default()));
    v.push(Box::new(SelectMessage::default()));
    v.push(Box::new(SelectContact::default()));
    v.push(Box::new(NormalMode::default()));
    v.push(Box::new(ComposeMode::default()));
    v.push(Box::new(SendMessage::default()));
    v.push(Box::new(React::default()));
    v.push(Box::new(Unreact::default()));
    v.push(Box::new(ReloadContacts::default()));
    v.push(Box::new(ReloadMessages::default()));
    v.push(Box::new(ReloadConfig::default()));
    v.push(Box::new(ComposeInEditor::default()));
    v.push(Box::new(ClearCompose::default()));
    v.push(Box::new(DownloadAttachments::default()));
    v.push(Box::new(OpenAttachments::default()));
    v.push(Box::new(OpenLink::default()));
    v.push(Box::new(MessageInfo::default()));
    v.push(Box::new(ContactInfo::default()));
    v.push(Box::new(Keybindings::default()));
    v.push(Box::new(Commands::default()));
    v.push(Box::new(CommandHistory::default()));
    v.push(Box::new(Reply::default()));
    v.push(Box::new(ScrollPopup::default()));
    v.push(Box::new(AttachFiles::default()));
    v.push(Box::new(DetachFiles::default()));
    v.push(Box::new(GotoQuoted::default()));
    v.push(Box::new(PipeMessage::default()));
    v.push(Box::new(Forward::default()));
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

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["quit"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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
        let last_selected = tui_state.contacts.state.selected();
        tui_state.contacts.state.select_next();
        after_contact_changed(tui_state, ba_tx, last_selected);
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["next-contact"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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
        let last_selected = tui_state.contacts.state.selected();
        tui_state.contacts.state.select_previous();
        after_contact_changed(tui_state, ba_tx, last_selected);
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["prev-contact"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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
        tui_state.messages.state.select_next();
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["next-message"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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
        tui_state.messages.state.select_previous();
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["prev-message"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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
                .messages
                .state
                .select(Some(num_messages - (abs_index % num_messages)));
        } else {
            tui_state.messages.state.select(Some(abs_index));
        }
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        let index = args
            .free_from_str()
            .map_err(|_e| Error::MissingArgument("index".to_owned()))?;
        *self = Self { index };
        check_unused_args(args)?;
        Ok(())
    }

    fn default() -> Self {
        Self { index: 0 }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["select-message"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self { index: self.index })
    }
}

#[derive(Debug)]
pub struct SelectContact {
    pub item: IndexOrString,
}

impl Command for SelectContact {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let last_selected = tui_state.contacts.state.selected();
        let index = match &self.item {
            IndexOrString::Index(index) => *index,
            IndexOrString::Str(name) => {
                let Some(index) = tui_state
                    .contacts
                    .iter_contacts_and_groups()
                    .position(|c| c.name.starts_with(name))
                else {
                    return Err(Error::InvalidArgument {
                        arg: "item".to_owned(),
                        value: name.to_owned(),
                    });
                };
                index
            }
        };

        tui_state.contacts.state.select(Some(index));

        after_contact_changed(tui_state, ba_tx, last_selected);

        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        let item = args
            .free_from_str()
            .map_err(|_e| Error::MissingArgument("item".to_owned()))?;
        self.item = item;
        check_unused_args(args)?;
        Ok(())
    }

    fn default() -> Self {
        Self {
            item: IndexOrString::Index(0),
        }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["select-contact"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self {
            item: self.item.clone(),
        })
    }

    fn complete(&self, tui_state: &TuiState, args: &str) -> Vec<Completion> {
        let mut names = tui_state
            .contacts
            .iter_contacts_and_groups()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>();
        let indices = (0..tui_state.contacts.len()).map(|i| i.to_string());
        names.extend(indices);
        complete_from_list(args, &names)
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
        tui_state.popup = None;
        tui_state.key_events.0.clear();
        tui_state.command_line.clear();
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["mode-normal"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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
        tui_state.mode = Mode::Command {
            previous: match tui_state.mode {
                Mode::Normal => crate::tui::BasicMode::Normal,
                Mode::Command { previous } => previous,
                Mode::Compose => crate::tui::BasicMode::Compose,
                Mode::Popup => crate::tui::BasicMode::Popup,
            },
        };
        tui_state.command_line.error.clear();
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["mode-command"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["mode-compose"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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
        let quoting = tui_state.compose.quote().clone();
        let attachments = tui_state.compose.attachments().to_vec();
        tui_state.compose.clear();
        NormalMode.execute(tui_state, ba_tx).unwrap();

        if message_body.is_empty() {
            return Ok(CommandSuccess::Nothing);
        }

        if let Some(contact) = tui_state.contacts.selected() {
            ba_tx
                .unbounded_send(BackendMessage::SendMessage {
                    contact_id: contact.id.clone(),
                    content: MessageContent::Text {
                        text: message_body,
                        attachments,
                    },
                    quote: quoting.map(|m| crate::backends::Quote {
                        timestamp: m.timestamp,
                        sender: m.sender,
                        text: m.text,
                    }),
                })
                .unwrap();
        }
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["send-message"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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

        let Some(contact) = tui_state.contacts.selected() else {
            return Err(Error::NoContactSelected);
        };

        let Some(selected_message) = tui_state.messages.selected() else {
            return Err(Error::NoMessageSelected);
        };

        ba_tx
            .unbounded_send(BackendMessage::SendMessage {
                contact_id: contact.id.clone(),
                content: MessageContent::Reaction {
                    message_author: selected_message.sender.clone(),
                    timestamp: selected_message.timestamp,
                    reaction: e.as_str().to_owned(),
                    remove: false,
                },
                quote: None,
            })
            .unwrap();
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        let emoji = args
            .free_from_str()
            .map_err(|_e| Error::MissingArgument("emoji".to_owned()))?;
        *self = Self { emoji };
        check_unused_args(args)?;
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

    fn complete(&self, _tui_state: &TuiState, args: &str) -> Vec<Completion> {
        let candidates = emojis::iter()
            .flat_map(|e| e.shortcodes())
            .map(|s| s.to_owned())
            .collect::<Vec<_>>();
        complete_from_list(args, &candidates)
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self {
            emoji: self.emoji.clone(),
        })
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
        let Some(contact) = tui_state.contacts.selected() else {
            return Err(Error::NoContactSelected);
        };

        let Some(selected_message) = tui_state.messages.selected() else {
            return Err(Error::NoMessageSelected);
        };

        let Some(reaction) = selected_message
            .reactions
            .iter()
            .find(|r| r.author == tui_state.self_id)
            .map(|r| r.emoji.clone())
        else {
            return Ok(CommandSuccess::Nothing);
        };

        ba_tx
            .unbounded_send(BackendMessage::SendMessage {
                contact_id: contact.id.clone(),
                content: MessageContent::Reaction {
                    message_author: selected_message.sender.clone(),
                    timestamp: selected_message.timestamp,
                    reaction,
                    remove: true,
                },
                quote: None,
            })
            .unwrap();
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["unreact"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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
        let cmdline = tui_state.command_line.text().to_owned();
        let previous_mode = match tui_state.mode {
            Mode::Normal => unreachable!(),
            Mode::Command { previous } => previous,
            Mode::Compose => unreachable!(),
            Mode::Popup => unreachable!(),
        };
        let mode = match previous_mode {
            crate::tui::BasicMode::Normal => Mode::Normal,
            crate::tui::BasicMode::Popup => Mode::Popup,
            crate::tui::BasicMode::Compose => Mode::Compose,
        };
        tui_state.mode = mode;
        // clear command
        tui_state.command_line.clear();

        tui_state.command_line.history.push(cmdline.clone());

        let args = shell_words::split(&cmdline)
            .unwrap()
            .into_iter()
            .map(OsString::from)
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
            command.parse(pargs)?;
            let ret = command.execute(tui_state, ba_tx)?;
            Ok(ret)
        } else {
            Err(Error::UnknownCommand(subcmd.to_owned()))
        }
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["execute-command"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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
        tui_state.contacts.state.select(None);
        ba_tx.unbounded_send(BackendMessage::LoadContacts).unwrap();
        Ok(CommandSuccess::Nothing)
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

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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
        tui_state.messages.state.select(None);
        if let Some(contact) = tui_state.contacts.selected() {
            ba_tx
                .unbounded_send(BackendMessage::LoadMessages {
                    contact_id: contact.id.clone(),
                    start_ts: std::ops::Bound::Unbounded,
                    end_ts: std::ops::Bound::Unbounded,
                })
                .unwrap();
        }
        Ok(CommandSuccess::Nothing)
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

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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
        let Some(contact) = tui_state.contacts.selected() else {
            return Err(Error::NoContactSelected);
        };

        let contact_name = contact.name.replace(" ", "_");

        let compose_content = tui_state.compose.lines().join("\n");
        let mut tmpfile = tempfile::Builder::new()
            .prefix(&format!("chatters-{}-", contact_name))
            .suffix(".txt")
            .tempfile()
            .unwrap();
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
            tui_state.compose.set_text(compose_lines);
        } else {
            warn!("Not using compose content from external editor due to error status");
        }
        Ok(CommandSuccess::Clear)
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

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
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
        tui_state.compose.clear();
        Ok(CommandSuccess::Nothing)
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

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
    }
}

#[derive(Debug)]
pub struct DownloadAttachments {
    // TODO: change to vec of indices
    item: Option<IndexOrString>,
}

impl Command for DownloadAttachments {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        if let Some(message) = tui_state.messages.selected() {
            let download_attachment =
                |message: &crate::tui::messages::Message,
                 attachment: &crate::backends::MessageAttachment| {
                    ba_tx
                        .unbounded_send(BackendMessage::DownloadAttachment {
                            contact_id: message.contact_id.clone(),
                            timestamp: message.timestamp,
                            index: attachment.index,
                        })
                        .unwrap();
                };
            match &self.item {
                Some(item) => {
                    let attachment = match item {
                        IndexOrString::Index(index) => message.attachments.get(*index),
                        IndexOrString::Str(name) => {
                            message.attachments.iter().find(|a| &a.name == name)
                        }
                    };

                    if let Some(attachment) = attachment {
                        download_attachment(&message, attachment)
                    }
                }
                None => {
                    for attachment in &message.attachments {
                        download_attachment(&message, attachment)
                    }
                }
            }
        }
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        let item = args.opt_free_from_str().unwrap();
        *self = Self { item };
        check_unused_args(args)?;
        Ok(())
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        Self { item: None }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["download-attachments"]
    }

    fn complete(&self, tui_state: &TuiState, args: &str) -> Vec<Completion> {
        let Some(message) = tui_state.messages.selected() else {
            return Vec::new();
        };
        let count = message.attachments.len();
        let mut indices = (0..count).map(|i| i.to_string()).collect::<Vec<_>>();
        let names = message.attachments.iter().map(|a| a.name.clone());
        indices.extend(names);
        complete_from_list(args, &indices)
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self {
            item: self.item.clone(),
        })
    }
}

#[derive(Debug)]
pub struct OpenAttachments {
    // TODO: change to vec of indices
    item: Option<IndexOrString>,
}

impl Command for OpenAttachments {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let Some(message) = tui_state.messages.selected() else {
            return Err(Error::NoMessageSelected);
        };
        let open_attachment = |path: &Option<PathBuf>| {
            if let Some(path) = path {
                debug!(path:? = path; "Opening attachment");
                open::that_detached(path).unwrap();
                Ok(())
            } else {
                // not downloaded yet
                Err(Error::Failure(
                    "Attachment has not been downloaded".to_owned(),
                ))
            }
        };
        match &self.item {
            Some(item) => {
                let attachment = match item {
                    IndexOrString::Index(index) => message.attachments.get(*index),
                    IndexOrString::Str(name) => {
                        message.attachments.iter().find(|a| &a.name == name)
                    }
                };
                if let Some(attachment) = attachment {
                    open_attachment(&attachment.path)?;
                }
            }
            None => {
                for attachment in &message.attachments {
                    open_attachment(&attachment.path)?;
                }
            }
        }
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        let item = args.opt_free_from_str().unwrap();
        *self = Self { item };
        check_unused_args(args)?;
        Ok(())
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        Self { item: None }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["open-attachments"]
    }

    fn complete(&self, tui_state: &TuiState, args: &str) -> Vec<Completion> {
        let Some(message) = tui_state.messages.selected() else {
            return Vec::new();
        };
        let count = message.attachments.len();
        let mut indices = (0..count).map(|i| i.to_string()).collect::<Vec<_>>();
        let names = message.attachments.iter().map(|a| a.name.clone());
        indices.extend(names);
        complete_from_list(args, &indices)
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self {
            item: self.item.clone(),
        })
    }
}

#[derive(Debug)]
pub struct OpenLink {
    item: IndexOrString,
}

static LINK_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        // from https://stackoverflow.com/a/63022807
        r"([\w+]+://)?([\w\d-]+\.)*[\w-]+[\.:]\w+([/?=&\#\.]?[\w-]+)*/?",
    )
    .unwrap()
});

impl Command for OpenLink {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let Some(message) = tui_state.messages.selected() else {
            return Err(Error::NoMessageSelected);
        };

        let mut links = LINK_REGEX.find_iter(&message.content).map(|m| m.as_str());

        let link = match &self.item {
            IndexOrString::Index(index) => {
                let Some(link) = links.nth(*index) else {
                    return Err(Error::Failure("Index past the number of links".to_owned()));
                };
                link
            }
            IndexOrString::Str(link) => link,
        };

        debug!(link:?; "Opening link");
        open::that(link).unwrap();

        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        let item = args
            .free_from_str()
            .map_err(|_e| Error::MissingArgument("item".to_owned()))?;
        *self = Self { item };
        check_unused_args(args)?;
        Ok(())
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        Self {
            item: IndexOrString::Index(0),
        }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["open-link"]
    }

    fn complete(&self, tui_state: &TuiState, args: &str) -> Vec<Completion> {
        let Some(message) = tui_state.messages.selected() else {
            return Vec::new();
        };
        let count = message.attachments.len();
        let mut indices = (0..count).map(|i| i.to_string()).collect::<Vec<_>>();
        let urls = LINK_REGEX
            .find_iter(&message.content)
            .map(|m| m.as_str().to_owned());
        indices.extend(urls);
        complete_from_list(args, &indices)
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self {
            item: self.item.clone(),
        })
    }
}

#[derive(Debug)]
pub struct MessageInfo;

impl Command for MessageInfo {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let Some(selected_message) = tui_state.messages.selected() else {
            return Err(Error::NoMessageSelected);
        };
        tui_state.popup = Some(Popup::new(PopupType::MessageInfo {
            timestamp: selected_message.timestamp,
        }));
        tui_state.mode = Mode::Popup;
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["message-info"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
    }
}

#[derive(Debug)]
pub struct ContactInfo;

impl Command for ContactInfo {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let Some(selected_contact) = tui_state.contacts.selected() else {
            return Err(Error::NoContactSelected);
        };
        tui_state.popup = Some(Popup::new(PopupType::ContactInfo {
            id: selected_contact.id.clone(),
        }));
        tui_state.mode = Mode::Popup;
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["contact-info"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
    }
}

#[derive(Debug)]
pub struct Keybindings;

impl Command for Keybindings {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.popup = Some(Popup::new(PopupType::Keybinds));
        tui_state.mode = Mode::Popup;
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["keybindings"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
    }
}

#[derive(Debug)]
pub struct Commands;

impl Command for Commands {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.popup = Some(Popup::new(PopupType::Commands));
        tui_state.mode = Mode::Popup;
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["commands"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
    }
}

#[derive(Debug)]
pub struct Reply;

impl Command for Reply {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let Some(selected_message) = tui_state.messages.selected() else {
            return Err(Error::NoMessageSelected);
        };
        tui_state.compose.set_quote(Quote {
            sender: selected_message.sender.clone(),
            timestamp: selected_message.timestamp,
            text: selected_message.content.clone(),
        });
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["reply"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
    }
}

#[derive(Debug)]
pub struct CommandHistory;

impl Command for CommandHistory {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.popup = Some(Popup::new(PopupType::CommandHistory));
        tui_state.mode = Mode::Popup;
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["command-history"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
    }
}

#[derive(Debug)]
pub struct PrevCommand;

impl Command for PrevCommand {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.command_line.history.select_previous();
        if let Some(selected_command) = tui_state.command_line.history.selected_command() {
            tui_state.command_line.set_text(selected_command.clone());
        } else {
            tui_state.command_line.clear();
        }
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["prev-command"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
    }
}

#[derive(Debug)]
pub struct NextCommand;

impl Command for NextCommand {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        tui_state.command_line.history.select_next();
        if let Some(selected_command) = tui_state.command_line.history.selected_command() {
            tui_state.command_line.set_text(selected_command.clone());
        } else {
            tui_state.command_line.clear();
        }
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["next-command"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
    }
}

#[derive(Debug)]
pub struct ScrollPopup {
    pub amount: i16,
}

impl Command for ScrollPopup {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        debug!(amount:% = self.amount; "Scrolling popup");
        let popup = tui_state.popup.as_mut().unwrap();
        if self.amount > 0 {
            popup.scroll += self.amount as u16;
        } else if self.amount < 0 {
            popup.scroll = popup.scroll.saturating_sub(self.amount.unsigned_abs());
        }
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        let amount = args
            .free_from_str()
            .map_err(|_e| Error::MissingArgument("amount".to_owned()))?;
        *self = Self { amount };
        check_unused_args(args)?;
        Ok(())
    }

    fn default() -> Self {
        Self { amount: 0 }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["scroll-popup"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self {
            amount: self.amount,
        })
    }
}

#[derive(Debug)]
pub struct AttachFiles {
    paths: Vec<String>,
}

impl Command for AttachFiles {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        if self.paths.is_empty() {
            return Err(Error::MissingArgument("path".to_owned()));
        }

        for path in &self.paths {
            let path = expand_tilde(path);

            if !path.is_file() {
                return Err(Error::InvalidArgument {
                    arg: "path".to_owned(),
                    value: path.to_string_lossy().into_owned(),
                });
            }
            tui_state.compose.attach_file(path.clone());
        }
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        loop {
            let path = args
                .opt_free_from_str()
                .map_err(|_e| Error::MissingArgument("paths".to_owned()))?;
            match path {
                Some(path) => {
                    self.paths.push(path);
                }
                None => break,
            }
        }
        check_unused_args(args)?;
        Ok(())
    }

    fn default() -> Self {
        Self { paths: Vec::new() }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["attach-files"]
    }

    fn complete(&self, _tui_state: &TuiState, args: &str) -> Vec<Completion> {
        let Some(path) = args.split(' ').last() else {
            return Vec::new();
        };

        complete_path(path)
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self {
            paths: self.paths.clone(),
        })
    }
}

#[derive(Debug)]
pub struct DetachFiles {
    indices: Vec<usize>,
}

impl Command for DetachFiles {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        if self.indices.is_empty() {
            return Err(Error::MissingArgument("index".to_owned()));
        }

        let mut indices = self.indices.clone();

        indices.sort();
        indices.reverse();

        for index in indices {
            tui_state.compose.detach_file(index);
        }

        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        loop {
            let index = args
                .opt_free_from_str()
                .map_err(|_e| Error::MissingArgument("indices".to_owned()))?;
            match index {
                Some(index) => {
                    self.indices.push(index);
                }
                None => break,
            }
        }
        check_unused_args(args)?;
        Ok(())
    }

    fn default() -> Self {
        Self {
            indices: Vec::new(),
        }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["detach-files"]
    }

    fn complete(&self, tui_state: &TuiState, args: &str) -> Vec<Completion> {
        let count = tui_state.compose.attachments().len();
        let candidates = (0..count).map(|i| i.to_string()).collect::<Vec<_>>();
        complete_from_list(args, &candidates)
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self {
            indices: self.indices.clone(),
        })
    }
}

#[derive(Debug)]
pub struct ReloadConfig;

impl Command for ReloadConfig {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let config = crate::util::load_config(&tui_state.config_path);
        tui_state.config = config;
        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["reload-config"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
    }
}

#[derive(Debug)]
pub struct GotoQuoted;

impl Command for GotoQuoted {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let Some(message) = tui_state.messages.selected() else {
            return Err(Error::NoMessageSelected);
        };
        let Some(quoted) = &message.quote else {
            return Err(Error::Failure("No quote to follow".to_owned()));
        };
        tui_state.messages.select_message(quoted.timestamp);

        Ok(CommandSuccess::Nothing)
    }

    fn default() -> Self {
        Self
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["goto-quoted"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self)
    }
}

#[derive(Debug)]
pub struct PipeMessage {
    command: String,
}

impl Command for PipeMessage {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        _ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let Some(message) = tui_state.messages.selected() else {
            return Err(Error::NoMessageSelected);
        };

        let mut child = std::process::Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(stdin, "{}", message.content).unwrap();
        child.wait().unwrap();

        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, args: pico_args::Arguments) -> Result<()> {
        let command = args
            .finish()
            .into_iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        self.command = command.join(" ");
        Ok(())
    }

    fn default() -> Self {
        Self {
            command: String::new(),
        }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["pipe-message"]
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self {
            command: self.command.clone(),
        })
    }
}

#[derive(Debug)]
pub struct Forward {
    contact_name: String,
}

impl Command for Forward {
    fn execute(
        &self,
        tui_state: &mut TuiState,
        ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    ) -> Result<CommandSuccess> {
        let Some(contact) = tui_state
            .contacts
            .contact_or_group_by_name(&self.contact_name)
        else {
            return Err(Error::InvalidArgument {
                arg: "contact".to_owned(),
                value: self.contact_name.clone(),
            });
        };

        let Some(selected_message) = tui_state.messages.selected() else {
            return Err(Error::NoMessageSelected);
        };

        ba_tx
            .unbounded_send(BackendMessage::SendMessage {
                contact_id: contact.id.clone(),
                content: MessageContent::Text {
                    text: selected_message.content.clone(),
                    attachments: selected_message.attachments.clone(),
                },
                quote: None,
            })
            .unwrap();
        Ok(CommandSuccess::Nothing)
    }

    fn parse(&mut self, mut args: pico_args::Arguments) -> Result<()> {
        let contact_name = args
            .free_from_str()
            .map_err(|_e| Error::MissingArgument("contact".to_owned()))?;
        *self = Self { contact_name };
        check_unused_args(args)?;
        Ok(())
    }

    fn default() -> Self {
        Self {
            contact_name: String::new(),
        }
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["forward"]
    }

    fn complete(&self, tui_state: &TuiState, args: &str) -> Vec<Completion> {
        let contact_name = args;

        let candidates = tui_state
            .contacts
            .iter_contacts_and_groups()
            .filter_map(|c| {
                if c.name.starts_with(contact_name) {
                    Some(shell_words::quote(&c.name).into_owned())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        complete_from_list(args, &candidates)
    }

    fn dyn_clone(&self) -> Box<dyn Command> {
        Box::new(Self {
            contact_name: self.contact_name.clone(),
        })
    }
}

fn after_contact_changed(
    tui_state: &mut TuiState,
    ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    last_selected: Option<usize>,
) {
    let selected = tui_state.contacts.state.selected();
    if selected == last_selected {
        return;
    }
    if let Some(contact) = tui_state.contacts.selected().cloned() {
        tui_state.messages.clear();
        tui_state.messages.state.select(None);
        ba_tx
            .unbounded_send(BackendMessage::LoadMessages {
                contact_id: contact.id.clone(),
                start_ts: std::ops::Bound::Unbounded,
                end_ts: std::ops::Bound::Unbounded,
            })
            .unwrap();
    }
}

fn check_unused_args(args: pico_args::Arguments) -> Result<()> {
    let unused_args = args.finish();
    if !unused_args.is_empty() {
        return Err(Error::UnknownArguments(
            unused_args
                .into_iter()
                .map(|s| s.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" "),
        ));
    }
    Ok(())
}

fn expand_tilde(s: &str) -> PathBuf {
    if s.starts_with("~") {
        let home = std::env::var("HOME").expect("HOME environment variable was not set");
        if s == "~" {
            return PathBuf::from(home);
        }
        let stripped_path = s.strip_prefix("~/").unwrap();
        let home_path = PathBuf::from(home);
        home_path.join(stripped_path)
    } else {
        PathBuf::from(s)
    }
}

pub fn complete_command(tui_state: &mut TuiState) {
    let cmd_line = tui_state.command_line.text_without_completion();
    if Some(&cmd_line) == tui_state.command_line.completions_generated_for().as_ref() {
        // reuse existing ones, select the next one
        tui_state.command_line.select_next_completion();
        return;
    }

    let cursor_index = tui_state.command_line.cursor_index();
    let before_cursor: String = cmd_line.chars().take(cursor_index).collect();

    let cmds = commands();
    let completions = if before_cursor.contains(' ') {
        let (subcmd, _rest) = before_cursor.split_once(' ').unwrap();
        let Some(command) = cmds.into_iter().find(|c| c.names().contains(&subcmd)) else {
            return;
        };
        command.complete(tui_state, &before_cursor)
    } else {
        debug!(before_cursor:?; "completing raw commands");
        complete_from_list(
            &before_cursor,
            &cmds
                .into_iter()
                .flat_map(|c| c.names())
                .map(|n| n.to_owned())
                .collect::<Vec<_>>(),
        )
    };
    if completions.len() == 1 {
        tui_state
            .command_line
            .append_text(completions[0].append.clone());
        tui_state.command_line.set_completions(Vec::new());
    } else {
        tui_state.command_line.set_completions(completions);
    }
}

#[derive(Debug, Clone)]
pub struct Completion {
    pub display: String,
    pub append: String,
}

fn last_part_of_shell_string(s: &str) -> String {
    let mut sofar = String::new();
    let mut inquote = false;
    let mut clear = false;
    for c in s.chars() {
        if clear {
            sofar.clear();
            clear = false;
        }

        if !inquote && c == ' ' {
            sofar.clear();
            continue;
        }

        sofar.push(c);

        let is_quote_char = matches!(c, '\'' | '"');
        if !inquote && is_quote_char {
            inquote = true;
        } else if inquote && is_quote_char {
            inquote = false;
            clear = true;
        }
    }
    sofar
}

fn complete_from_list(cmd_line: &str, list: &[String]) -> Vec<Completion> {
    let last_part = last_part_of_shell_string(cmd_line);

    let result = list
        .iter()
        .map(|li| shell_words::quote(li))
        .filter_map(|li| {
            if li.starts_with(&last_part) {
                Some(Completion {
                    append: li.strip_prefix(&last_part).unwrap().to_owned(),
                    display: li.into_owned(),
                })
            } else {
                None
            }
        })
        .collect();
    debug!(cmd_line:?, list:?, result:?; "Completed from list");
    result
}

fn complete_path(current: &str) -> Vec<Completion> {
    let path = if current.is_empty() {
        current_dir().unwrap()
    } else {
        expand_tilde(current)
    };

    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    debug!(path:?, file_name:?, is_dir:? = path.is_dir(); "Getting completions for path");

    let candidates = if path.is_dir() {
        read_dir(&path)
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                e.path().to_string_lossy().into_owned()
            })
            .collect::<Vec<_>>()
    } else if let Some(path) = path.parent() {
        let Ok(entries) = read_dir(path) else {
            return Vec::new();
        };
        entries
            .map(|e| {
                let e = e.unwrap();
                e.path().to_string_lossy().into_owned()
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    complete_from_list(&path.to_string_lossy(), &candidates)
}

#[derive(Debug, Clone)]
pub enum IndexOrString {
    Index(usize),
    Str(String),
}

impl FromStr for IndexOrString {
    type Err = Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.parse::<usize>() {
            Ok(num) => Ok(Self::Index(num)),
            Err(_) => Ok(Self::Str(s.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_completion() {
        let list = ["foo".to_owned(), "bar".to_owned(), "baz".to_owned()];
        insta::assert_debug_snapshot!(complete_from_list("f", &list));
        insta::assert_debug_snapshot!(complete_from_list("foo", &list));
        insta::assert_debug_snapshot!(complete_from_list("b", &list));
        insta::assert_debug_snapshot!(complete_from_list("bar", &list));
    }

    #[test]
    fn test_last_part_of_shell_string() {
        insta::assert_debug_snapshot!(last_part_of_shell_string("abc"));
        insta::assert_debug_snapshot!(last_part_of_shell_string("'abc"));
        insta::assert_debug_snapshot!(last_part_of_shell_string("'abc'"));
        insta::assert_debug_snapshot!(last_part_of_shell_string("'abc' foo"));
        insta::assert_debug_snapshot!(last_part_of_shell_string("'abc' 'foo"));
        insta::assert_debug_snapshot!(last_part_of_shell_string("'abc' 'foo'"));
        insta::assert_debug_snapshot!(last_part_of_shell_string("abc foo"));
    }

    #[test]
    fn test_index_or_string_from_str() {
        insta::assert_debug_snapshot!(IndexOrString::from_str("1"));
        insta::assert_debug_snapshot!(IndexOrString::from_str("+1"));
        insta::assert_debug_snapshot!(IndexOrString::from_str("/test/"));
        insta::assert_debug_snapshot!(IndexOrString::from_str("12/test/"));
    }
}
