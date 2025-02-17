use crate::{hooks::Hooks, keybinds::KeyBinds};

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub hooks: Hooks,
    pub keybinds: KeyBinds,
}
