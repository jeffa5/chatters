use crate::keybinds::KeyBinds;

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub keybinds: KeyBinds,
}
