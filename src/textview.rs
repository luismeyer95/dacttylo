use std::collections::HashMap;
use std::ops::Range;

use syntect::highlighting::Style;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    text::StyledGrapheme,
    widgets::{Block, StatefulWidget, Widget},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::line_processor::LineProcessor;
use crate::utils;
use crate::utils::reflow::{LineComposer, WordWrapper};
use crate::{
    highlight::{Highlighter, NoHighlight, SyntectHighlight},
    line_stylizer::LineStylizer,
};

pub enum Anchor {
    Start(usize),
    End(usize),
}
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct TextCoord(pub usize, pub usize);

pub struct TextView<'a> {
    /// The full text buffer
    text_lines: Vec<&'a str>,

    /// Controls the view offset behaviour
    anchor: Anchor,

    line_processor: Box<dyn LineProcessor>,

    /// Sparse styling applied after the syntax highlight pass,
    /// used for cursors and special application logic highlighting
    sparse_styling: HashMap<TextCoord, tui::style::Style>,

    /// Enclosing block component
    block: Block<'a>,

    /// Option to override the background color after all styles are applied
    bg_color: tui::style::Color,

    /// Optional closure to set external UI state from the list of displayed lines
    /// on render
    metadata_handler: Option<Box<dyn Fn(Range<usize>) + 'a>>,
}

impl<'a> TextView<'a> {
    /// Instantiate a TextView widget from a line buffer and use the builder
    /// pattern to set custom rendering options
    pub fn new(text_lines: Vec<&'a str>) -> Self {
        Self {
            text_lines,
            line_processor: Box::new(
                LineStylizer::new().syntax_styling(Box::new(SyntectHighlight::new())),
            ),
            anchor: Anchor::Start(0),
            sparse_styling: HashMap::new(),
            block: Default::default(),
            bg_color: tui::style::Color::Reset,
            metadata_handler: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block;
        self
    }

    pub fn line_processor(mut self, line_processor: Box<dyn LineProcessor>) -> Self {
        self.line_processor = line_processor;
        self
    }

    pub fn anchor(mut self, anchor: Anchor) -> Self {
        match &anchor {
            Anchor::Start(anchor) if *anchor >= self.text_lines.len() => {
                panic!("anchor out of bounds")
            }
            Anchor::End(anchor) if *anchor > self.text_lines.len() => {
                panic!("anchor out of bounds")
            }
            _ => {}
        }
        self.anchor = anchor;
        self
    }

    pub fn sparse_styling(mut self, sparse_styling: HashMap<TextCoord, tui::style::Style>) -> Self {
        self.sparse_styling = sparse_styling;
        self
    }

    pub fn bg_color(mut self, color: tui::style::Color) -> Self {
        self.bg_color = color;
        self
    }

    /// Pass a callback to this function to set external UI state.
    /// The callback is passed
    /// - a vector of line heights (acts as a map from line number to row count)
    /// - the height of the text view render buffer
    pub fn on_wrap(mut self, callback: Box<dyn Fn(Range<usize>) + 'a>) -> Self {
        self.metadata_handler = Some(callback);
        self
    }

    fn render_block(&mut self, area: &mut Rect, buf: &mut Buffer) {
        let block = std::mem::take(&mut self.block);

        // save the inner_area because render consumes the block
        let inner_area = block.inner(*area);
        block.render(*area, buf);

        *area = inner_area;
    }

    fn process_view(&mut self, area: Rect) -> Vec<Vec<StyledGrapheme<'_>>> {
        match self.anchor {
            Anchor::Start(anchor) => self.process_anchor_start(anchor, area),
            Anchor::End(anchor) => self.process_anchor_end(anchor, area),
        }
    }

    fn process_anchor_start(&mut self, anchor: usize, area: Rect) -> Vec<Vec<StyledGrapheme<'_>>> {
        let lines = std::mem::take(&mut self.text_lines);
        let mut rows: Vec<Vec<StyledGrapheme<'_>>> = vec![];
        let mut current_ln = anchor;

        while current_ln < lines.len() {
            let mut line_as_rows = self.line_to_rows(current_ln, lines[current_ln], &area);
            if line_as_rows.len() + rows.len() > area.height as usize {
                break;
            }
            rows.extend(line_as_rows);
            current_ln += 1;
        }

        // passing the actually displayed line range
        if let Some(metadata_handler) = &self.metadata_handler {
            metadata_handler(anchor..current_ln);
        }

        rows
    }

    fn extract_ln_styling(
        map: &HashMap<TextCoord, tui::style::Style>,
        ln_offset: usize,
    ) -> HashMap<usize, tui::style::Style> {
        map.iter()
            .filter_map(|(coord, &style)| (coord.0 == ln_offset).then(|| (coord.1, style)))
            .collect()
    }

    fn line_to_rows<'txt>(
        &mut self,
        line_nb: usize,
        line: &'txt str,
        area: &Rect,
    ) -> Vec<Vec<StyledGrapheme<'txt>>> {
        let styling = Self::extract_ln_styling(&self.sparse_styling, line_nb);
        self.line_processor.process_line(line, styling, area.width)
    }

    fn process_anchor_end(&mut self, anchor: usize, area: Rect) -> Vec<Vec<StyledGrapheme<'_>>> {
        let lines = std::mem::take(&mut self.text_lines);
        let mut rows: Vec<Vec<StyledGrapheme<'_>>> = vec![];
        let mut current_ln = anchor - 1;

        loop {
            let mut line_as_rows = self.line_to_rows(current_ln, lines[current_ln], &area);
            if line_as_rows.len() + rows.len() > area.height as usize {
                break;
            }

            line_as_rows.extend(rows);
            rows = line_as_rows;
            match current_ln.checked_sub(1) {
                Some(next) => current_ln = next,
                None => break,
            }
        }

        let start_ln = current_ln + 1;

        current_ln = anchor;
        while current_ln < lines.len() {
            let mut line_as_rows = self.line_to_rows(current_ln, lines[current_ln], &area);
            if line_as_rows.len() + rows.len() > area.height as usize {
                break;
            }
            rows.extend(line_as_rows);
            current_ln += 1;
        }

        let end_ln = current_ln;

        // passing the actually displayed line range
        if let Some(metadata_handler) = &self.metadata_handler {
            metadata_handler(start_ln..end_ln);
        }

        rows
    }
}

impl<'a> Widget for TextView<'a> {
    fn render(mut self, mut area: Rect, buf: &mut Buffer) {
        self.render_block(&mut area, buf);
        if area.height < 1 || area.width < 1 {
            return;
        }

        let bg_style = tui::style::Style::default().bg(self.bg_color);
        for y in 0..area.height {
            for x in 0..area.width {
                let cell = buf.get_mut(area.left() + x, area.top() + y);
                cell.set_style(cell.style().patch(bg_style));
            }
        }

        let lines = self.process_view(area);
        let mut y = 0;
        for line in lines {
            let mut x = 0;
            for StyledGrapheme { symbol, style } in line {
                buf.get_mut(area.left() + x, area.top() + y)
                    .set_symbol(if symbol.is_empty() { " " } else { symbol })
                    .set_style(style);
                x += symbol.width() as u16;
            }
            y += 1;
            if y >= area.height {
                break;
            }
        }
    }
}

////////////////////////////////////////////////////////////

pub enum ViewCommand {
    SetStart(usize),
    SetEnd(usize),
    ShiftUntil(usize),
    CenterOn(usize),
}

pub struct RenderMetadata {
    /// Height of the last render buffer
    pub buffer_height: u16,
    /// Mapping from line number to total number of rows after wrapping
    pub line_rows_map: Vec<usize>,
}

pub struct EditorView<'a> {
    /// Full linesplit text buffer, only a subset will be rendered each frame
    pub text_lines: Vec<&'a str>,

    /// The current line offset to use for rendering
    pub anchor: usize,

    /// The view command to process on the next render
    pub command: ViewCommand,

    /// Metadata on the previous frame to compute the next frame
    pub last_render: Option<RenderMetadata>,
}

impl<'a> EditorView<'a> {
    pub fn new(text_lines: Vec<&'a str>) -> Self {
        Self {
            text_lines,
            anchor: 0,
            command: ViewCommand::SetStart(0),
            last_render: None,
        }
    }

    pub fn command(&mut self, cmd: ViewCommand) {
        self.command = cmd;
    }

    pub fn renderer(&self) -> EditorRenderer {
        EditorRenderer
    }

    fn compute_next_anchor(&mut self, area: &Rect) -> usize {
        todo!();
    }
}

pub struct EditorRenderer;

impl<'a> StatefulWidget for &'a EditorRenderer {
    type State = EditorView<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let new_anchor = match state.last_render.as_ref() {
            Some(last_render) => state.compute_next_anchor(&area),
            None => state.anchor,
        };

        let lines = state.text_lines[new_anchor..].to_vec();

        // let typeview = TextView::new(lines)
        //     .bg_color(Color::Rgb(0, 27, 46))
        //     .sparse_styling(HashMap::<usize, tui::style::Style>::from_iter(vec![(
        //         0,
        //         tui::style::Style::default()
        //             .bg(Color::White)
        //             .fg(Color::Black),
        //     )]));
        // typeview.render(area, buf);
    }
}
