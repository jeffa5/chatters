#[derive(Debug, Default)]
pub struct CommandLineHistory {
    commands: Vec<String>,
    selected: Option<usize>,
}

impl CommandLineHistory {
    pub fn push(&mut self, command_line: String) {
        self.commands.push(command_line);
    }

    pub fn selected_command(&self) -> Option<&String> {
        self.selected.map(|i| &self.commands[i])
    }

    pub fn select_previous(&mut self) {
        if let Some(selected) = self.selected {
            self.selected = Some(selected.saturating_sub(1))
        } else if !self.commands.is_empty() {
            self.selected = Some(self.commands.len() - 1)
        }
    }

    pub fn select_next(&mut self) {
        if let Some(selected) = self.selected {
            if selected + 1 < self.commands.len() {
                self.selected = Some(selected + 1);
            } else {
                self.selected = None
            }
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.commands.iter()
    }
}
