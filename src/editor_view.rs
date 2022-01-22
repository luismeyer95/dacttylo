use crate::highlight::Highlighter;
use crate::text_view::RenderMetadata;
use crate::{
    highlight::SyntectHighlight,
    text_coord::TextCoord,
    text_view::{Anchor, TextView},
};
use std::{collections::HashMap, ops::Range};
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
        // TODO: remove!
        let mut hl = SyntectHighlight::new();
        self.text_lines = lines
            .into_iter()
            .map(|s| {
                hl.highlight_line(s)
                    .into_iter()
                    .map(|(tkn, color)| {
                        (tkn, tui::style::Style::default().fg(color))
                    })
                    .collect()
            })
            .collect();
        self

        // self.text_lines = lines
        //     .into_iter()
        //     .map(|s| vec![(s, tui::style::Style::default())])
        //     .collect();
        // self
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
                // if
                if state.focus_coord.ln >= lines_rendered.end {
                    Anchor::End(state.focus_coord.ln + 1)
                } else if state.focus_coord.ln < lines_rendered.start {
                    Anchor::Start(state.focus_coord.ln)
                } else {
                    Anchor::Start(lines_rendered.start)
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

        // let eggshell = Color::Rgb(255, 239, 214);
        let darkblue = Color::Rgb(0, 27, 46);

        let view = TextView::new()
            .styled_content(self.text_lines)
            .anchor(anchor)
            .on_render(|metadata| {
                state.last_render = Some(metadata);
            })
            .bg_color(darkblue)
            .sparse_styling(
                HashMap::<TextCoord, tui::style::Style>::from_iter(vec![(
                    TextCoord::new(state.focus_coord.ln, state.focus_coord.x),
                    tui::style::Style::default()
                        .bg(Color::White)
                        .fg(Color::Black),
                )]),
            );
        view.render(area, buf);
    }
}
