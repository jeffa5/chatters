use std::collections::HashMap;

use crossterm::event::KeyCode;

use crate::commands::{
    Command, CommandMode, ComposeMode, ExecuteCommand, NextContact, NextMessage, NormalMode,
    PrevContact, PrevMessage, Quit, SendMessage,
};

#[derive(Debug)]
pub struct KeyBinds {
    pub bindings: HashMap<KeyCode, Box<dyn Command>>,
}

impl KeyBinds {
    pub fn normal_default() -> Self {
        let mut bindings: HashMap<KeyCode, Box<dyn Command>> = HashMap::new();
        bindings.insert(KeyCode::Char('q'), Box::new(Quit));
        bindings.insert(KeyCode::Char('J'), Box::new(NextContact));
        bindings.insert(KeyCode::Char('K'), Box::new(PrevContact));
        bindings.insert(KeyCode::Char('j'), Box::new(NextMessage));
        bindings.insert(KeyCode::Char('k'), Box::new(PrevMessage));
        bindings.insert(KeyCode::Char(':'), Box::new(CommandMode));
        bindings.insert(KeyCode::Char('i'), Box::new(ComposeMode));
        Self { bindings }
    }

    pub fn command_default() -> Self {
        let mut bindings: HashMap<KeyCode, Box<dyn Command>> = HashMap::new();
        bindings.insert(KeyCode::Esc, Box::new(NormalMode));
        bindings.insert(KeyCode::Enter, Box::new(ExecuteCommand));
        Self { bindings }
    }

    pub fn compose_default() -> Self {
        let mut bindings: HashMap<KeyCode, Box<dyn Command>> = HashMap::new();
        bindings.insert(KeyCode::Esc, Box::new(NormalMode));
        bindings.insert(KeyCode::Enter, Box::new(SendMessage));
        Self { bindings }
    }

    pub fn get(&self, s: &KeyCode) -> Option<&Box<dyn Command>> {
        self.bindings.get(s)
    }
}
