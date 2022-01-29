use crate::highlighting::Highlighter;
use crate::text_view::RenderMetadata;
use crate::{
    highlighting::SyntectHighlighter,
    text_coord::TextCoord,
    text_view::{Anchor, TextView},
};
use std::collections::HashMap;
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
pub struct EditorRenderer<'a, 'cb> {
    /// Full linesplit text buffer, only a subset will be rendered each frame
    // pub text_lines: Vec<StyledLineIterator<'a>>,
    text_view: TextView<'a, 'cb>,
}

impl<'a, 'cb> EditorRenderer<'a, 'cb> {
    pub fn styled_content<Lns, Ln>(lines: Lns) -> Self
    where
        Lns: Iterator<Item = Ln>,
        Ln: Into<Box<dyn Iterator<Item = StyledGrapheme<'a>> + 'a>>,
    {
        Self {
            text_view: TextView::new().styled_content(lines),
        }
    }

    pub fn content<Lns>(lines: Lns) -> Self
    where
        Lns: Iterator<Item = &'a str>,
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

// impl<'a, 'cb> Default for EditorRenderer<'a, 'cb> {
//     fn default() -> Self {
//         Self::new()
//     }
// }

impl<'a, 'cb> StatefulWidget for EditorRenderer<'a, 'cb> {
    type State = EditorViewState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let anchor = Self::compute_anchor(state);
        let cursor_style = tui::style::Style::default()
            .bg(Color::Black)
            .fg(Color::White);

        // let eggshell = Color::Rgb(255, 239, 214);
        // let darkblue = Color::Rgb(0, 27, 46);

        let view = self
            .text_view
            .anchor(anchor)
            .on_render(|metadata| {
                state.last_render = Some(metadata);
            })
            .sparse_styling(
                HashMap::<TextCoord, tui::style::Style>::from_iter(vec![(
                    TextCoord::new(state.focus_coord.ln, state.focus_coord.x),
                    cursor_style,
                )]),
            );
        view.render(area, buf);
    }
}
