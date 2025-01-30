use crossterm::event::KeyEvent;
use ratatui::{style::Style, widgets::Block};
use tui_textarea::TextArea;

#[derive(Debug, Default)]
pub struct Compose {
    textarea: TextArea<'static>,
}

impl Compose {
    pub fn set_text(&mut self, lines: Vec<String>) {
        self.textarea = TextArea::new(lines);
    }

    pub fn lines(&self) -> &[String] {
        self.textarea.lines()
    }

    pub fn set_block(&mut self, block: Block<'static>) {
        self.textarea.set_block(block);
    }

    pub fn set_cursor_style(&mut self, style: Style) {
        self.textarea.set_cursor_style(style);
    }

    pub fn set_cursor_line_style(&mut self, style: Style) {
        self.textarea.set_cursor_line_style(style);
    }

    pub fn input(&mut self, key_event: KeyEvent) {
        self.textarea.input(key_event);
    }

    pub fn clear(&mut self) {
        self.textarea = TextArea::default();
    }
}

impl ratatui::widgets::Widget for &Compose {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        self.textarea.render(area, buf)
    }
}
