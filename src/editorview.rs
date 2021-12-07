use crate::textview::{Anchor, TextCoord, TextView};
use std::{cell::Cell, collections::HashMap, ops::Range};
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    widgets::{StatefulWidget, Widget},
};

type StyledLine<'a> = Vec<(&'a str, tui::style::Style)>;
pub struct EditorView<'a> {
    /// Full linesplit text buffer, only a subset will be rendered each frame
    pub text_lines: Vec<StyledLine<'a>>,

    /// The current line offset to use for rendering
    pub anchor: Cell<usize>,
    pub last_render: Cell<Option<Range<usize>>>,

    pub focus_line: usize,
}

impl<'a> EditorView<'a> {
    pub fn new() -> Self {
        Self {
            text_lines: vec![],
            anchor: 0.into(),
            last_render: None.into(),
            focus_line: 0,
        }
    }

    pub fn content(mut self, lines: Vec<&'a str>) -> Self {
        self.text_lines = lines
            .into_iter()
            .map(|s| vec![(s, tui::style::Style::default())])
            .collect();
        self
    }

    pub fn styled_content(mut self, lines: Vec<StyledLine<'a>>) -> Self {
        self.text_lines = lines;
        self
    }

    pub fn focus(mut self, line: usize) -> Self {
        self.focus_line = line;
        self
    }

    pub fn renderer(&self) -> EditorRenderer {
        EditorRenderer
    }
}

pub struct EditorRenderer;

impl<'a> StatefulWidget for &'a EditorRenderer {
    type State = EditorView<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let anchor = match state.last_render.take() {
            Some(render) => {
                if state.focus_line >= render.end {
                    Anchor::End(state.focus_line + 1)
                } else if state.focus_line < render.start {
                    Anchor::Start(state.focus_line)
                } else {
                    Anchor::Start(state.anchor.get())
                }
            }
            None => Anchor::Start(state.anchor.get()),
        };

        let typeview = TextView::new()
            .styled_content(state.text_lines.clone())
            .anchor(anchor)
            .on_wrap(Box::new(|displayed_lines| {
                state.anchor.set(displayed_lines.start);
                state.last_render.set(Some(displayed_lines));
            }))
            .bg_color(Color::Rgb(0, 27, 46))
            .sparse_styling(HashMap::<TextCoord, tui::style::Style>::from_iter(vec![(
                TextCoord(state.focus_line, 0),
                tui::style::Style::default()
                    .bg(Color::White)
                    .fg(Color::Black),
            )]));
        typeview.render(area, buf);
    }
}
