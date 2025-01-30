use crossterm::event::KeyEvent;
use presage::libsignal_service::prelude::Uuid;
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Paragraph},
};
use tui_textarea::TextArea;

#[derive(Debug, Default)]
pub struct Compose {
    textarea: TextArea<'static>,
    block: Block<'static>,
    reply: Option<Reply>,
}

#[derive(Debug, Clone)]
pub struct Reply {
    pub sender: Uuid,
    pub timestamp: u64,
    pub text: String,
}

impl Compose {
    pub fn set_text(&mut self, lines: Vec<String>) {
        self.textarea = TextArea::new(lines);
    }

    pub fn set_reply(&mut self, reply: Reply) {
        self.reply = Some(reply);
    }

    pub fn reply(&self) -> &Option<Reply> {
        &self.reply
    }

    pub fn lines(&self) -> &[String] {
        self.textarea.lines()
    }

    pub fn set_block(&mut self, block: Block<'static>) {
        self.block = block;
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
        self.reply = None;
    }

    pub fn height(&self) -> u16 {
        self.reply.as_ref().map_or(0, |_| 1) + self.textarea.lines().len().max(1) as u16 + 1
        // 1 for top border
    }
}

impl ratatui::widgets::Widget for &Compose {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut constraints = Vec::new();

        let mut textarea_index = 0;

        if self.reply.is_some() {
            constraints.push(Constraint::Length(1));
            textarea_index += 1;
        }
        constraints.push(Constraint::Length(self.textarea.lines().len().max(1) as u16));

        log::debug!(self:?, area:?; "Rendering compose");

        self.block.clone().render(area.clone(), buf);
        let area = self.block.inner(area);
        let vertical = Layout::vertical(constraints).split(area);

        if let Some(reply) = &self.reply {
            let reply_text = reply.text.lines().next().unwrap();
            let reply_text = format!("> {reply_text}");
            Paragraph::new(reply_text).render(vertical[0], buf);
        }

        self.textarea.render(vertical[textarea_index], buf)
    }
}
