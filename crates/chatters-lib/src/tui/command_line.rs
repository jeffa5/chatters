use crossterm::event::KeyEvent;
use tui_textarea::TextArea;

use crate::{command_history::CommandLineHistory, commands::Completion};

use super::list::ListState;

#[derive(Debug, Default)]
pub struct CommandLine {
    command: TextArea<'static>,
    pub error: String,
    pub completions: Completions,
    pub history: CommandLineHistory,
}

impl CommandLine {
    pub fn text(&self) -> &str {
        self.command.lines().first().unwrap()
    }

    pub fn text_without_completion(&self) -> String {
        if let Some(completion) = self
            .completions
            .list_state
            .selected()
            .map(|i| self.completions.candidates[i].clone())
        {
            let mut textarea = self.command.clone();
            let char_count = completion.append.chars().count();
            for _ in 0..char_count {
                textarea.move_cursor(tui_textarea::CursorMove::Back);
            }
            textarea.delete_str(char_count);
            textarea.lines().first().unwrap().to_owned()
        } else {
            self.text().to_owned()
        }
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
        self.completions.clear();
        self.command.input(key_event);
    }

    pub fn completions(&self) -> &[Completion] {
        self.completions.candidates()
    }

    pub fn set_completions(&mut self, completions: Vec<Completion>) {
        self.completions
            .set_completions(completions, self.text_without_completion());
    }

    pub fn selected_completion(&self) -> Option<usize> {
        self.completions.list_state.selected()
    }

    pub fn textarea(&mut self) -> &mut TextArea<'static> {
        &mut self.command
    }

    pub fn select_next_completion(&mut self) {
        let last = self
            .completions
            .list_state
            .selected()
            .map(|i| self.completions.candidates[i].clone());
        self.completions.select_next();
        if let Some(last) = last {
            let char_count = last.append.chars().count();
            for _ in 0..char_count {
                self.command.move_cursor(tui_textarea::CursorMove::Back);
            }
            self.command.delete_str(char_count);
        }
        if let Some(comp) = self
            .completions
            .list_state
            .selected()
            .map(|i| self.completions.candidates[i].clone())
        {
            self.command.insert_str(comp.append);
        }
    }

    pub fn completions_generated_for(&self) -> &Option<String> {
        &self.completions.generated_for
    }
}

#[derive(Debug, Default)]
pub struct Completions {
    pub candidates: Vec<Completion>,
    pub list_state: ListState,
    pub generated_for: Option<String>,
}

impl Completions {
    pub fn candidates(&self) -> &[Completion] {
        &self.candidates
    }

    pub fn set_completions(&mut self, completions: Vec<Completion>, generated_for: String) {
        if Some(&generated_for) != self.generated_for.as_ref() {
            self.list_state.select(None);
            self.candidates = completions;
            self.generated_for = Some(generated_for);
        }
    }

    pub fn clear(&mut self) {
        self.candidates.clear();
        self.list_state.select(None);
        self.generated_for = None;
    }

    fn select_next(&mut self) {
        if self.candidates.is_empty() {
            return;
        }
        if let Some(index) = self.list_state.selected() {
            let new_index = index + 1;
            if new_index == self.candidates.len() {
                self.list_state.select(None);
            } else {
                self.list_state.select(Some(new_index));
            }
        } else {
            self.list_state.select(Some(0));
        }
    }
}
