use std::collections::BTreeMap;

use crate::commands::{Command, NextContact, NextMessage, PrevContact, PrevMessage, Quit};

#[derive(Debug)]
pub struct KeyBinds {
    pub bindings: BTreeMap<String, Box<dyn Command>>,
}

impl Default for KeyBinds {
    fn default() -> Self {
        let mut bindings: BTreeMap<String, Box<dyn Command>> = BTreeMap::new();
        bindings.insert("q".to_owned(), Box::new(Quit));
        bindings.insert("J".to_owned(), Box::new(NextContact));
        bindings.insert("K".to_owned(), Box::new(PrevContact));
        bindings.insert("j".to_owned(), Box::new(NextMessage));
        bindings.insert("k".to_owned(), Box::new(PrevMessage));
        Self { bindings }
    }
}

impl KeyBinds {
    pub fn get(&self, s: &str) -> Option<&Box<dyn Command>> {
        self.bindings.get(s)
    }
}
