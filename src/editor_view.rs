use crate::highlighting::Highlighter;
use crate::text_view::RenderMetadata;
use crate::{
    highlighting::SyntectHighlighter,
    text_coord::TextCoord,
    text_view::{Anchor, TextView},
};
use std::collections::HashMap;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    widgets::{StatefulWidget, Widget},
};

type StyledLine<'a> = Vec<(&'a str, tui::style::Style)>;

pub struct EditorViewState {
    /// The current line offset to use for rendering
    // pub anchor: usize,
    pub last_render: Option<RenderMetadata>,

    /// The coord to keep in display range
    pub focus_coord: TextCoord,
}

impl EditorViewState {
    pub fn new() -> Self {
        Self {
            // anchor: 0,
            last_render: None,
            focus_coord: TextCoord::new(0, 0),
        }
    }

    pub fn focus(&mut self, coord: TextCoord) {
        self.focus_coord = coord;
    }
}

impl Default for EditorViewState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EditorRenderer<'a> {
    /// Full linesplit text buffer, only a subset will be rendered each frame
    pub text_lines: Vec<StyledLine<'a>>,
}

impl<'a> EditorRenderer<'a> {
    pub fn new() -> Self {
        Self { text_lines: vec![] }
    }

    pub fn content(mut self, lines: Vec<&'a str>) -> Self {
        // TODO: works for now but inefficient!
        let mut hl = SyntectHighlighter::new().extension("rs").build().unwrap();
        self.text_lines = hl.highlight(lines.as_ref());
        self
    }

    pub fn styled_content(mut self, lines: Vec<StyledLine<'a>>) -> Self {
        self.text_lines = lines;
        self
    }

    fn compute_anchor(state: &mut EditorViewState) -> Anchor {
        match state.last_render.take() {
            Some(RenderMetadata {
                lines_rendered,
                anchor,
            }) => {
                if lines_rendered.is_empty() {
                    anchor
                } else if state.focus_coord.ln >= lines_rendered.end {
                    Anchor::End(state.focus_coord.ln + 1)
                } else if state.focus_coord.ln < lines_rendered.start {
                    Anchor::Start(state.focus_coord.ln)
                } else {
                    anchor
                }
            }
            None => Anchor::Start(0),
        }
    }
}

impl<'a> Default for EditorRenderer<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> StatefulWidget for EditorRenderer<'a> {
    type State = EditorViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let anchor = Self::compute_anchor(state);
        let cursor_style = tui::style::Style::default()
            .bg(Color::Black)
            .fg(Color::White);

        // let eggshell = Color::Rgb(255, 239, 214);
        // let darkblue = Color::Rgb(0, 27, 46);

        let view = TextView::new()
            .styled_content(self.text_lines)
            .anchor(anchor)
            .on_render(|metadata| {
                state.last_render = Some(metadata);
            })
            // .bg_color(darkblue)
            .sparse_styling(
                HashMap::<TextCoord, tui::style::Style>::from_iter(vec![(
                    TextCoord::new(state.focus_coord.ln, state.focus_coord.x),
                    cursor_style,
                )]),
            );
        view.render(area, buf);
    }
}
