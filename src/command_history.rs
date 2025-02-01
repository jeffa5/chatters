use crate::commands::Command;

#[derive(Debug, Default)]
pub struct CommandHistory {
    commands: Vec<Box<dyn Command>>,
    selected: Option<usize>,
}

impl CommandHistory {
    pub fn push(&mut self, command: Box<dyn Command>) {
        self.commands.push(command)
    }

    pub fn selected_command(&self) -> Option<&dyn Command> {
        self.selected.map(|i| &*self.commands[i])
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

    pub fn iter(&self) -> impl Iterator<Item = &Box<dyn Command>> {
        self.commands.iter()
    }
}
