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

#[derive(Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl Display for KeyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.modifiers == KeyModifiers::NONE || matches!(self.code, KeyCode::Char(_)) {
            write!(f, "{}", self.code)
        } else {
            write!(f, "{}-{}", self.modifiers, self.code)
        }
    }
}

type KeyEvents = Vec<KeyEvent>;

#[derive(Debug)]
pub struct KeyBinds {
    pub normal_bindings: Vec<(KeyEvents, Box<dyn Command>)>,
    pub command_bindings: Vec<(KeyEvents, Box<dyn Command>)>,
    pub compose_bindings: Vec<(KeyEvents, Box<dyn Command>)>,
    pub popup_bindings: Vec<(KeyEvents, Box<dyn Command>)>,
}

impl Default for KeyBinds {
    fn default() -> Self {
        let mut normal = Vec::<(KeyEvents, Box<dyn Command>)>::new();
        normal.push((vec![char('q')], Box::new(Quit)));
        normal.push((vec![char('J')], Box::new(NextContact)));
        normal.push((vec![code_shift(KeyCode::Down)], Box::new(NextContact)));
        normal.push((vec![char('K')], Box::new(PrevContact)));
        normal.push((vec![code_shift(KeyCode::Up)], Box::new(PrevContact)));
        normal.push((vec![char('j')], Box::new(NextMessage)));
        normal.push((vec![code(KeyCode::Down)], Box::new(NextMessage)));
        normal.push((vec![char('k')], Box::new(PrevMessage)));
        normal.push((vec![code(KeyCode::Up)], Box::new(PrevMessage)));
        normal.push((vec![char(':')], Box::new(CommandMode)));
        normal.push((vec![char('i')], Box::new(ComposeMode)));
        normal.push((vec![char('g')], Box::new(SelectMessage { index: 0 })));
        normal.push((vec![char('G')], Box::new(SelectMessage { index: -1 })));
        normal.push((vec![char('I')], Box::new(ComposeInEditor)));
        normal.push((vec![code(KeyCode::Enter)], Box::new(SendMessage)));
        normal.push((vec![char('?')], Box::new(Keybindings)));
        normal.push((vec![char('h')], Box::new(CommandHistory)));

        let mut command = Vec::<(KeyEvents, Box<dyn Command>)>::new();
        command.push((vec![code(KeyCode::Esc)], Box::new(NormalMode)));
        command.push((vec![code(KeyCode::Up)], Box::new(PrevCommand)));
        command.push((vec![code(KeyCode::Down)], Box::new(NextCommand)));
        command.push((vec![code(KeyCode::Enter)], Box::new(ExecuteCommand)));

        let mut compose = Vec::<(KeyEvents, Box<dyn Command>)>::new();
        compose.push((vec![code(KeyCode::Esc)], Box::new(NormalMode)));

        let mut popup = Vec::<(KeyEvents, Box<dyn Command>)>::new();
        popup.push((vec![code(KeyCode::Esc)], Box::new(NormalMode)));
        popup.push((vec![char(':')], Box::new(CommandMode)));
        popup.push((vec![char('j')], Box::new(ScrollPopup { amount: 1 })));
        popup.push((vec![char('k')], Box::new(ScrollPopup { amount: -1 })));

        Self {
            normal_bindings: normal,
            command_bindings: command,
            compose_bindings: compose,
            popup_bindings: popup,
        }
    }
}

impl KeyBinds {
    pub fn get(&self, events: &[KeyEvent], mode: Mode) -> Result<&Box<dyn Command>, bool> {
        let bindings = match mode {
            Mode::Normal => &self.normal_bindings,
            Mode::Command { .. } => &self.command_bindings,
            Mode::Compose => &self.compose_bindings,
            Mode::Popup => &self.popup_bindings,
        };
        let mut prefix = false;
        for (keys, command) in bindings {
            if keys == events {
                return Ok(command);
            }
            if keys.starts_with(events) {
                prefix = true;
            }
        }
        Err(prefix)
    }

    pub fn iter(&self, mode: Mode) -> impl Iterator<Item = &(KeyEvents, Box<dyn Command>)> {
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
        modifiers: if c.is_uppercase() {
            KeyModifiers::SHIFT
        } else {
            KeyModifiers::NONE
        },
    }
}

fn code(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
    }
}

fn code_shift(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::SHIFT,
    }
}
