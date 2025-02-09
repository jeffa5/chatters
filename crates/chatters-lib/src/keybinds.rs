use std::{fmt::Display, str::FromStr};

use crossterm::event::{KeyCode, KeyModifiers};

use crate::tui::Mode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl Display for KeyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let is_plain_char = matches!(self.code, KeyCode::Char(_)) && self.modifiers.is_empty();
        let is_capital = matches!(self.code, KeyCode::Char(c) if c.is_uppercase());
        let is_plain_capital = is_capital && self.modifiers == KeyModifiers::SHIFT;

        if is_plain_char || is_plain_capital {
            write!(f, "{}", self.code)
        } else {
            write!(f, "<")?;
            if self.modifiers.contains(KeyModifiers::CONTROL) {
                write!(f, "C-")?;
            }
            if self.modifiers.contains(KeyModifiers::SHIFT) && !is_capital {
                write!(f, "S-")?;
            }
            if self.modifiers.contains(KeyModifiers::ALT) {
                write!(f, "A-")?;
            }
            write!(f, "{}>", self.code)
        }
    }
}

impl FromStr for KeyEvent {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut slf = Self {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
        };
        if s.starts_with("<") && s.ends_with(">") {
            let mut inner = &s[1..s.len() - 1];

            loop {
                if let Some(remainder) = inner.strip_prefix("c-") {
                    slf.modifiers |= KeyModifiers::CONTROL;
                    inner = remainder;
                    continue;
                }
                if let Some(remainder) = inner.strip_prefix("C-") {
                    slf.modifiers |= KeyModifiers::CONTROL;
                    inner = remainder;
                    continue;
                }
                if let Some(remainder) = inner.strip_prefix("a-") {
                    slf.modifiers |= KeyModifiers::ALT;
                    inner = remainder;
                    continue;
                }
                if let Some(remainder) = inner.strip_prefix("A-") {
                    slf.modifiers |= KeyModifiers::ALT;
                    inner = remainder;
                    continue;
                }
                if let Some(remainder) = inner.strip_prefix("s-") {
                    slf.modifiers |= KeyModifiers::SHIFT;
                    inner = remainder;
                    continue;
                }
                if let Some(remainder) = inner.strip_prefix("S-") {
                    slf.modifiers |= KeyModifiers::SHIFT;
                    inner = remainder;
                    continue;
                }
                break;
            }

            slf.code = if inner.len() == 1 {
                let c = inner.chars().next().unwrap();
                if c.is_uppercase() {
                    slf.modifiers |= KeyModifiers::SHIFT;
                }
                KeyCode::Char(c)
            } else {
                match inner.to_lowercase().as_str() {
                    "enter" => KeyCode::Enter,
                    "esc" => KeyCode::Esc,
                    "up" => KeyCode::Up,
                    "down" => KeyCode::Down,
                    "left" => KeyCode::Left,
                    "right" => KeyCode::Right,
                    "home" => KeyCode::Home,
                    "end" => KeyCode::End,
                    "pageup" => KeyCode::PageUp,
                    "pagedown" => KeyCode::PageDown,
                    _ => return Err(()),
                }
            };
        } else if s.len() == 1 {
            let c = s.chars().next().unwrap();
            if c.is_uppercase() {
                slf.modifiers |= KeyModifiers::SHIFT;
            }
            slf.code = KeyCode::Char(c)
        } else {
            return Err(());
        }
        Ok(slf)
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct KeyEvents(pub Vec<KeyEvent>);

impl FromStr for KeyEvents {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut slf = Self(Vec::new());
        let mut ke = String::new();
        for c in s.chars() {
            if c == '<' {
                ke.push(c);
            } else if c == '>' {
                ke.push(c);
                let key_event = KeyEvent::from_str(&ke)?;
                slf.0.push(key_event);
                ke.clear();
            } else if ke.is_empty() {
                slf.0.push(KeyEvent::from_str(&c.to_string())?);
            } else {
                ke.push(c);
            }
        }
        Ok(slf)
    }
}

impl Display for KeyEvents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for event in &self.0 {
            write!(f, "{}", event)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct KeyBinds {
    pub normal_bindings: Vec<(KeyEvents, String)>,
    pub command_bindings: Vec<(KeyEvents, String)>,
    pub compose_bindings: Vec<(KeyEvents, String)>,
    pub popup_bindings: Vec<(KeyEvents, String)>,
}

impl Default for KeyBinds {
    fn default() -> Self {
        let mut normal = Vec::<(KeyEvents, String)>::new();
        normal.push((KeyEvents::from_str("q").unwrap(), ":quit<enter>".to_owned()));
        normal.push((
            KeyEvents::from_str("J").unwrap(),
            ":next-contact<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("<s-down>").unwrap(),
            ":next-contact<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("K").unwrap(),
            ":prev-contact<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("<s-up>").unwrap(),
            ":prev-contact<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("j").unwrap(),
            ":next-message<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("<down>").unwrap(),
            ":next-message<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("k").unwrap(),
            ":prev-message<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("<up>").unwrap(),
            ":prev-message<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("i").unwrap(),
            ":mode-compose<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("g").unwrap(),
            ":select-message 0<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("G").unwrap(),
            ":select-message -1<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("I").unwrap(),
            ":compose-in-editor<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("<enter>").unwrap(),
            ":send-message<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("?").unwrap(),
            ":keybindings<enter>".to_owned(),
        ));
        normal.push((
            KeyEvents::from_str("h").unwrap(),
            ":command-history<enter>".to_owned(),
        ));

        let mut popup = Vec::<(KeyEvents, String)>::new();
        popup.push((
            KeyEvents::from_str("j").unwrap(),
            ":scroll-popup 1<enter>".to_owned(),
        ));
        popup.push((
            KeyEvents::from_str("k").unwrap(),
            ":scroll-popup -1<enter>".to_owned(),
        ));

        Self {
            normal_bindings: normal,
            command_bindings: Vec::new(),
            compose_bindings: Vec::new(),
            popup_bindings: popup,
        }
    }
}

impl KeyBinds {
    pub fn get(&self, events: &KeyEvents, mode: Mode) -> Result<&String, bool> {
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
            if keys.0.starts_with(&events.0) {
                prefix = true;
            }
        }
        Err(prefix)
    }

    pub fn iter(&self, mode: Mode) -> impl Iterator<Item = &(KeyEvents, String)> {
        match mode {
            Mode::Normal => &self.normal_bindings,
            Mode::Command { .. } => &self.command_bindings,
            Mode::Compose => &self.compose_bindings,
            Mode::Popup => &self.popup_bindings,
        }
        .iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_key_events() {
        insta::assert_debug_snapshot!(KeyEvents::from_str("a<enter><c-esc>"));
    }

    #[test]
    fn parse_key_event() {
        insta::assert_debug_snapshot!(KeyEvent::from_str("a"));
        insta::assert_debug_snapshot!(KeyEvent::from_str("<c-a>"));
        insta::assert_debug_snapshot!(KeyEvent::from_str("<s-c-a>"));
        insta::assert_debug_snapshot!(KeyEvent::from_str("<esc>"));
    }

    #[test]
    fn display_key_event() {
        insta::assert_debug_snapshot!(vec![
            KeyEvent::from_str("a").unwrap().to_string(),
            KeyEvent::from_str("A").unwrap().to_string(),
            KeyEvent::from_str("<c-a>").unwrap().to_string(),
            KeyEvent::from_str("<c-A>").unwrap().to_string(),
            KeyEvent::from_str("<up>").unwrap().to_string(),
            KeyEvent::from_str("<c-up>").unwrap().to_string(),
        ]);
    }
}
