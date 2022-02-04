use crate::highlighting::Highlighter;
use crate::text_view::RenderMetadata;
use crate::{
    highlighting::SyntectHighlighter,
    text_coord::TextCoord,
    text_view::{Anchor, TextView},
};
use std::collections::HashMap;
use std::iter;
use std::ops::Deref;
use tui::text::StyledGrapheme;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    widgets::{StatefulWidget, Widget},
};

// type StyledLine<'a> = Vec<(&'a str, tui::style::Style)>;

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

type StyledLineIterator<'a> = Box<dyn Iterator<Item = StyledGrapheme<'a>> + 'a>;
pub struct EditorRenderer<'a> {
    /// Full linesplit text buffer, only a subset will be rendered each frame
    // pub text_lines: Vec<StyledLineIterator<'a>>,
    text_view: TextView<'a>,
}

impl<'a> EditorRenderer<'a> {
    pub fn styled_content<Lns, Ln>(lines: Lns) -> Self
    where
        Lns: Iterator<Item = Ln>,
        Ln: Into<Box<dyn Iterator<Item = StyledGrapheme<'a>> + 'a>>,
    {
        Self {
            text_view: TextView::new().styled_content(lines),
        }
    }

    pub fn content<Lns, Ref>(lines: Lns) -> Self
    where
        Lns: IntoIterator<Item = Ref>,
        Ref: Deref<Target = &'a str>,
    {
        Self {
            text_view: TextView::new().content(lines),
        }
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

impl<'a> StatefulWidget for EditorRenderer<'a> {
    type State = EditorViewState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let anchor = Self::compute_anchor(state);

        // let eggshell = Color::Rgb(255, 239, 214);
        // let darkblue = Color::Rgb(0, 27, 46);

        let cursor = iter::once((
            TextCoord::new(state.focus_coord.ln, state.focus_coord.x),
            tui::style::Style::default()
                .bg(Color::Black)
                .fg(Color::White),
        ));

        let view = self
            .text_view
            .anchor(anchor)
            .sparse_styling(HashMap::<_, _>::from_iter(cursor));

        view.render(area, buf, &mut state.last_render);
    }
}
