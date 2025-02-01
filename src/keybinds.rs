use std::fmt::Display;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::commands::{
    Command, CommandHistory, CommandMode, ComposeInEditor, ComposeMode, ExecuteCommand,
    Keybindings, NextCommand, NextContact, NextMessage, NormalMode, PopupScroll, PrevCommand,
    PrevContact, PrevMessage, Quit, SelectMessage, SendMessage,
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
    pub bindings: Vec<(KeyEvent, Box<dyn Command>)>,
}

impl KeyBinds {
    pub fn normal_default() -> Self {
        let mut bindings = Vec::<(KeyEvent, Box<dyn Command>)>::new();
        bindings.push((char('q'), Box::new(Quit)));
        bindings.push((char('J'), Box::new(NextContact)));
        bindings.push((code_shift(KeyCode::Down), Box::new(NextContact)));
        bindings.push((char('K'), Box::new(PrevContact)));
        bindings.push((code_shift(KeyCode::Up), Box::new(PrevContact)));
        bindings.push((char('j'), Box::new(NextMessage)));
        bindings.push((code(KeyCode::Down), Box::new(NextMessage)));
        bindings.push((char('k'), Box::new(PrevMessage)));
        bindings.push((code(KeyCode::Up), Box::new(PrevMessage)));
        bindings.push((char(':'), Box::new(CommandMode)));
        bindings.push((char('i'), Box::new(ComposeMode)));
        bindings.push((char('g'), Box::new(SelectMessage { index: 0 })));
        bindings.push((char('G'), Box::new(SelectMessage { index: -1 })));
        bindings.push((char('I'), Box::new(ComposeInEditor)));
        bindings.push((code(KeyCode::Enter), Box::new(SendMessage)));
        bindings.push((char('?'), Box::new(Keybindings)));
        bindings.push((char('h'), Box::new(CommandHistory)));
        Self { bindings }
    }

    pub fn command_default() -> Self {
        let mut bindings = Vec::<(KeyEvent, Box<dyn Command>)>::new();
        bindings.push((code(KeyCode::Esc), Box::new(NormalMode)));
        bindings.push((code(KeyCode::Up), Box::new(PrevCommand)));
        bindings.push((code(KeyCode::Down), Box::new(NextCommand)));
        bindings.push((any_code(KeyCode::Enter), Box::new(ExecuteCommand)));
        Self { bindings }
    }

    pub fn compose_default() -> Self {
        let mut bindings = Vec::<(KeyEvent, Box<dyn Command>)>::new();
        bindings.push((code(KeyCode::Esc), Box::new(NormalMode)));
        Self { bindings }
    }

    pub fn popup_default() -> Self {
        let mut bindings = Vec::<(KeyEvent, Box<dyn Command>)>::new();
        bindings.push((code(KeyCode::Esc), Box::new(NormalMode)));
        bindings.push((char('j'), Box::new(PopupScroll { amount: 1 })));
        bindings.push((char('k'), Box::new(PopupScroll { amount: -1 })));
        Self { bindings }
    }

    pub fn get(&self, code: KeyCode, mods: KeyModifiers) -> Option<&Box<dyn Command>> {
        let key_event = &KeyEvent {
            code,
            modifiers: KeyMods::Modifiers(mods),
        };
        self.bindings
            .iter()
            .find_map(|(ke, c)| if ke == key_event { Some(c) } else { None })
    }

    pub fn iter(&self) -> impl Iterator<Item = &(KeyEvent, Box<dyn Command>)> {
        self.bindings.iter()
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
