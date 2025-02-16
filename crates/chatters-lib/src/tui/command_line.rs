use crossterm::event::KeyEvent;
use tui_textarea::TextArea;

use crate::{command_history::CommandLineHistory, commands::Completion};

#[derive(Debug, Default)]
pub struct CommandLine {
    command: TextArea<'static>,
    pub error: String,
    completions: Vec<Completion>,
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

    pub fn append_text(&mut self, text: String) {
        self.command.insert_str(text);
    }

    pub fn clear(&mut self) {
        self.command = TextArea::default();
        self.error.clear();
        self.completions.clear();
        self.history.clear_selection();
    }

    pub fn cursor_index(&self) -> usize {
        self.command.cursor().1
    }

    pub fn input(&mut self, key_event: KeyEvent) {
        self.command.input(key_event);
    }

    pub fn completions(&self) -> &[Completion] {
        &self.completions
    }

    pub fn set_completions(&mut self, mut completions: Vec<Completion>) {
        completions.sort_by(|c1, c2| c1.display.cmp(&c2.display));
        self.completions = completions;
    }

    pub fn textarea(&mut self) -> &mut TextArea<'static> {
        &mut self.command
    }
}
