use ratatui::{
    layout::Rect,
    style::Style,
    text::Text,
    widgets::{Clear, StatefulWidget, Widget},
};

#[derive(Debug, Default)]
pub struct ListState {
    /// First item to render.
    offset: usize,

    selected: Option<usize>,
}

impl ListState {
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn select_next(&mut self) {
        self.selected = Some(self.selected.map_or(0, |i| i + 1));
    }

    pub fn select_previous(&mut self) {
        if let Some(selected) = self.selected {
            self.selected = Some(selected.saturating_sub(1));
        }
    }

    pub fn select_last(&mut self) {
        self.selected = Some(usize::MAX);
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}

#[derive(Debug, Default)]
pub struct VerticalList {
    items: Vec<Text<'static>>,

    selected_item_style: Style,
}

impl VerticalList {
    pub fn new(items: Vec<Text<'static>>) -> Self {
        Self {
            items,
            selected_item_style: Style::new(),
        }
    }

    pub fn set_selected_item_style(&mut self, style: Style) {
        self.selected_item_style = style;
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl StatefulWidget for &VerticalList {
    type State = ListState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        Clear.render(area, buf);

        if let Some(mut selected) = state.selected {
            if selected >= self.len() {
                selected = self.len() - 1;
                state.selected = Some(selected);
            }
            state.offset = state.offset.min(selected);

            loop {
                let height = self
                    .items
                    .iter()
                    .skip(state.offset)
                    .map(|t| t.lines.len())
                    .sum::<usize>();
                if height < area.height as usize && state.offset > 0 {
                    state.offset -= 1;
                    continue;
                }
                break;
            }

            loop {
                let height = self
                    .items
                    .iter()
                    .skip(state.offset)
                    .map(|t| t.lines.len())
                    .take(selected.saturating_sub(state.offset).saturating_add(1))
                    .sum::<usize>();
                if height > area.height as usize {
                    state.offset += 1;
                    continue;
                }
                break;
            }
        } else {
            // set it to the start of the list
            state.offset = 0;
        }

        // now actually draw the list
        let mut used_height = 0;
        for (i, item) in self.items.iter().enumerate().skip(state.offset) {
            if used_height >= area.height {
                // no space left to draw
                break;
            }
            let item_height = item.lines.len() as u16;

            let mut text = item.clone();
            if Some(i) == state.selected {
                text = text.style(self.selected_item_style);
            }

            text.render(
                Rect {
                    y: area.y + used_height,
                    height: item_height,
                    ..area
                },
                buf,
            );

            used_height += item_height;
        }
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    use super::*;

    #[test]
    fn test_vertical_list() {
        let list = VerticalList::new(vec![
            "single line".into(),
            "multi\nline\nstring".into(),
            "a\nb\nc\n1\n2\n3\n4\n5\n6\n7".into(),
        ]);
        let mut state = ListState::default();

        let mut terminal = Terminal::new(TestBackend::new(20, 10)).unwrap();
        terminal
            .draw(|frame| frame.render_stateful_widget(&list, frame.area(), &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());

        state.select_last();
        terminal
            .draw(|frame| frame.render_stateful_widget(&list, frame.area(), &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());

        state.select_previous();
        terminal
            .draw(|frame| frame.render_stateful_widget(&list, frame.area(), &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_vertical_list_keep_large_list_filling_screen() {
        let list = VerticalList {
            items: (0..20).map(|i| Text::from(i.to_string())).collect(),
            selected_item_style: Style::new(),
        };
        let mut state = ListState {
            offset: 15,
            selected: Some(20),
        };
        let mut terminal = Terminal::new(TestBackend::new(20, 10)).unwrap();
        terminal
            .draw(|frame| frame.render_stateful_widget(&list, frame.area(), &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
