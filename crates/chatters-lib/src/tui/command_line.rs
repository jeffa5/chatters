use crossterm::event::KeyEvent;
use tui_textarea::TextArea;

use crate::command_history::CommandLineHistory;

#[derive(Debug, Default)]
pub struct CommandLine {
    command: TextArea<'static>,
    pub error: String,
    completions: Vec<String>,
    pub history: CommandLineHistory,
}

impl CommandLine {
    pub fn text(&self) -> &str {
        self.command.lines().first().unwrap()
    }

    pub fn set_text(&mut self, text: String) {
        self.command = TextArea::new(vec![text]);
        self.command.move_cursor(tui_textarea::CursorMove::End);
    }

    pub fn clear(&mut self) {
        self.command = TextArea::default();
        self.error.clear();
        self.completions.clear();
        self.history.clear_selection();
    }

    pub fn input(&mut self, key_event: KeyEvent) {
        self.command.input(key_event);
    }

    pub fn completions(&self) -> &[String] {
        &self.completions
    }

    pub fn set_completions(&mut self, mut completions: Vec<String>) {
        completions.sort();
        self.completions = completions;
    }

    pub fn textarea(&mut self) -> &mut TextArea<'static> {
        &mut self.command
    }
}
