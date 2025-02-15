use std::{os::unix::fs::MetadataExt as _, path::PathBuf};

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Paragraph},
};
use tui_textarea::TextArea;

use crate::backends::MessageAttachment;

use super::messages::Quote;

#[derive(Debug, Default)]
pub struct Compose {
    textarea: TextArea<'static>,
    block: Block<'static>,
    quote: Option<Quote>,
    attachments: Vec<MessageAttachment>,
}

impl Compose {
    pub fn set_text(&mut self, lines: Vec<String>) {
        self.textarea = TextArea::new(lines);
    }

    pub fn set_quote(&mut self, quote: Quote) {
        self.quote = Some(quote);
    }

    pub fn quote(&self) -> &Option<Quote> {
        &self.quote
    }

    pub fn attachments(&self) -> &[MessageAttachment] {
        &self.attachments
    }

    pub fn attach_file(&mut self, path: PathBuf) {
        // TODO: check for duplicate path
        self.attachments.push(MessageAttachment {
            name: path.file_name().unwrap().to_string_lossy().into_owned(),
            index: 0,
            size: path.metadata().map(|m| m.size()).unwrap_or(0),
            path: Some(path),
        })
    }

    pub fn detach_file(&mut self, index: usize) {
        if index >= self.attachments.len() {
            return;
        }
        self.attachments.remove(index);
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
        self.quote = None;
        self.attachments.clear();
    }

    pub fn height(&self) -> u16 {
        self.quote.as_ref().map_or(0, |_| 1)
            + self.attachments.len() as u16
            + self.textarea.lines().len().max(1) as u16
            + 1
        // 1 for top border
    }
}

impl ratatui::widgets::Widget for &Compose {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut constraints = Vec::new();

        let mut attachments_index = 0;
        let mut textarea_index = 0;

        if self.quote.is_some() {
            constraints.push(Constraint::Length(1));
            attachments_index += 1;
            textarea_index += 1;
        }
        if !self.attachments.is_empty() {
            constraints.push(Constraint::Length(self.attachments.len() as u16));
            textarea_index += self.attachments.len();
        }
        constraints.push(Constraint::Length(self.textarea.lines().len().max(1) as u16));

        self.block.clone().render(area, buf);
        let area = self.block.inner(area);
        let vertical = Layout::vertical(constraints).split(area);

        if let Some(quote) = &self.quote {
            let quote_text = quote.text.lines().next().unwrap();
            let quote_text = format!("> {quote_text}");
            Paragraph::new(quote_text).render(vertical[0], buf);
        }

        if !self.attachments.is_empty() {
            let mut lines = Vec::new();
            for attachment in &self.attachments {
                lines.push(ratatui::text::Line::from(attachment.message_line()));
            }
            Paragraph::new(lines).render(vertical[attachments_index], buf);
        }

        self.textarea.render(vertical[textarea_index], buf)
    }
}
