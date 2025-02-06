use std::fmt::Display;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    commands::{
        Command, CommandHistory, CommandMode, ComposeInEditor, ComposeMode, ExecuteCommand,
        Keybindings, NextCommand, NextContact, NextMessage, NormalMode, PrevCommand, PrevContact,
        PrevMessage, Quit, ScrollPopup, SelectMessage, SendMessage,
    },
    tui::Mode,
};

#[derive(Debug)]
pub enum KeyMods {
    Modifiers(KeyModifiers),
    Any,
}

impl Display for KeyMods {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyMods::Modifiers(key_modifiers) => {
                write!(f, "{}", key_modifiers)
            }
            KeyMods::Any => write!(f, "Any"),
        }
    }
}

impl PartialEq for KeyMods {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Modifiers(l0), Self::Modifiers(r0)) => l0 == r0,
            (Self::Any, Self::Modifiers(_)) => true,
            (Self::Modifiers(_), Self::Any) => true,
            (Self::Any, Self::Any) => true,
        }
    }
}

impl Eq for KeyMods {}

#[derive(Debug, PartialEq, Eq)]
pub struct KeyEvent {
    code: KeyCode,
    modifiers: KeyMods,
}

impl Display for KeyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.modifiers == KeyMods::Modifiers(KeyModifiers::NONE)
            || matches!(self.code, KeyCode::Char(_))
        {
            write!(f, "{}", self.code)
        } else {
            write!(f, "{}+{}", self.modifiers, self.code)
        }
    }
}

#[derive(Debug)]
pub struct KeyBinds {
    pub normal_bindings: Vec<(KeyEvent, Box<dyn Command>)>,
    pub command_bindings: Vec<(KeyEvent, Box<dyn Command>)>,
    pub compose_bindings: Vec<(KeyEvent, Box<dyn Command>)>,
    pub popup_bindings: Vec<(KeyEvent, Box<dyn Command>)>,
}

impl Default for KeyBinds {
    fn default() -> Self {
        let mut normal = Vec::<(KeyEvent, Box<dyn Command>)>::new();
        normal.push((char('q'), Box::new(Quit)));
        normal.push((char('J'), Box::new(NextContact)));
        normal.push((code_shift(KeyCode::Down), Box::new(NextContact)));
        normal.push((char('K'), Box::new(PrevContact)));
        normal.push((code_shift(KeyCode::Up), Box::new(PrevContact)));
        normal.push((char('j'), Box::new(NextMessage)));
        normal.push((code(KeyCode::Down), Box::new(NextMessage)));
        normal.push((char('k'), Box::new(PrevMessage)));
        normal.push((code(KeyCode::Up), Box::new(PrevMessage)));
        normal.push((char(':'), Box::new(CommandMode)));
        normal.push((char('i'), Box::new(ComposeMode)));
        normal.push((char('g'), Box::new(SelectMessage { index: 0 })));
        normal.push((char('G'), Box::new(SelectMessage { index: -1 })));
        normal.push((char('I'), Box::new(ComposeInEditor)));
        normal.push((code(KeyCode::Enter), Box::new(SendMessage)));
        normal.push((char('?'), Box::new(Keybindings)));
        normal.push((char('h'), Box::new(CommandHistory)));

        let mut command = Vec::<(KeyEvent, Box<dyn Command>)>::new();
        command.push((code(KeyCode::Esc), Box::new(NormalMode)));
        command.push((code(KeyCode::Up), Box::new(PrevCommand)));
        command.push((code(KeyCode::Down), Box::new(NextCommand)));
        command.push((any_code(KeyCode::Enter), Box::new(ExecuteCommand)));

        let mut compose = Vec::<(KeyEvent, Box<dyn Command>)>::new();
        compose.push((code(KeyCode::Esc), Box::new(NormalMode)));

        let mut popup = Vec::<(KeyEvent, Box<dyn Command>)>::new();
        popup.push((code(KeyCode::Esc), Box::new(NormalMode)));
        popup.push((char(':'), Box::new(CommandMode)));
        popup.push((char('j'), Box::new(ScrollPopup { amount: 1 })));
        popup.push((char('k'), Box::new(ScrollPopup { amount: -1 })));

        Self {
            normal_bindings: normal,
            command_bindings: command,
            compose_bindings: compose,
            popup_bindings: popup,
        }
    }
}

impl KeyBinds {
    pub fn get(&self, code: KeyCode, mods: KeyModifiers, mode: Mode) -> Option<&Box<dyn Command>> {
        let key_event = &KeyEvent {
            code,
            modifiers: KeyMods::Modifiers(mods),
        };
        match mode {
            Mode::Normal => &self.normal_bindings,
            Mode::Command { .. } => &self.command_bindings,
            Mode::Compose => &self.compose_bindings,
            Mode::Popup => &self.popup_bindings,
        }
        .iter()
        .find_map(|(ke, c)| if ke == key_event { Some(c) } else { None })
    }

    pub fn iter(&self, mode: Mode) -> impl Iterator<Item = &(KeyEvent, Box<dyn Command>)> {
        match mode {
            Mode::Normal => &self.normal_bindings,
            Mode::Command { .. } => &self.command_bindings,
            Mode::Compose => &self.compose_bindings,
            Mode::Popup => &self.popup_bindings,
        }
        .iter()
    }
}

fn char(c: char) -> KeyEvent {
    KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyMods::Modifiers(if c.is_uppercase() {
            KeyModifiers::SHIFT
        } else {
            KeyModifiers::NONE
        }),
    }
}

fn code(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyMods::Modifiers(KeyModifiers::NONE),
    }
}

fn code_shift(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyMods::Modifiers(KeyModifiers::SHIFT),
    }
}

fn any_code(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyMods::Any,
    }
}
