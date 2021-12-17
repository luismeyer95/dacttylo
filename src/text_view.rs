use std::collections::HashMap;
use std::ops::Range;
use tui::{
    buffer::Buffer,
    layout::Rect,
    text::StyledGrapheme,
    widgets::{Block, Widget},
};
use unicode_width::UnicodeWidthStr;

use crate::line_stylizer::LineStylizer;
use crate::{line_processor::LineProcessor, text_coord::TextCoord};

type StyledLine<'a> = Vec<(&'a str, tui::style::Style)>;

pub enum Anchor {
    Start(usize),
    End(usize),
}

pub struct TextView<'a> {
    /// The full text buffer
    text_lines: Vec<StyledLine<'a>>,

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
    metadata_handler: Option<Box<dyn FnMut(Range<usize>) + 'a>>,
}

impl<'a> TextView<'a> {
    /// Instantiate a TextView widget from a line buffer and use the builder
    /// pattern to set custom rendering options
    pub fn new() -> Self {
        Self {
            text_lines: vec![],
            line_processor: Box::new(LineStylizer),
            anchor: Anchor::Start(0),
            sparse_styling: HashMap::new(),
            block: Default::default(),
            bg_color: tui::style::Color::Reset,
            metadata_handler: None,
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
    pub fn on_wrap(mut self, callback: Box<dyn FnMut(Range<usize>) + 'a>) -> Self {
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

    fn generate_view(&mut self, area: Rect) -> Vec<Vec<StyledGrapheme<'_>>> {
        match self.anchor {
            Anchor::Start(anchor) => self.generate_start_anchor(anchor, area),
            Anchor::End(anchor) => self.generate_end_anchor(anchor, area),
        }
    }

    fn generate_rows_down<'txt>(
        &mut self,
        current_ln: &mut usize,
        lines: &[Vec<(&'txt str, tui::style::Style)>],
        area: &Rect,
    ) -> Vec<Vec<StyledGrapheme<'txt>>> {
        let mut rows: Vec<Vec<StyledGrapheme<'_>>> = vec![];

        while *current_ln < lines.len() {
            let line_as_rows = self.line_to_rows(*current_ln, &lines[*current_ln], &area);
            if line_as_rows.len() + rows.len() > area.height as usize {
                break;
            }
            rows.extend(line_as_rows);
            *current_ln += 1;
        }

        rows
    }

    fn generate_rows_up<'txt>(
        &mut self,
        current_ln: &mut usize,
        lines: &[Vec<(&'txt str, tui::style::Style)>],
        area: &Rect,
    ) -> Vec<Vec<StyledGrapheme<'txt>>> {
        let mut rows: Vec<Vec<StyledGrapheme<'_>>> = vec![];

        *current_ln = loop {
            let mut line_as_rows = self.line_to_rows(*current_ln, &lines[*current_ln], &area);
            if line_as_rows.len() + rows.len() > area.height as usize {
                break *current_ln + 1;
            }
            line_as_rows.extend(rows);
            rows = line_as_rows;
            match current_ln.checked_sub(1) {
                Some(next) => *current_ln = next,
                None => break 0,
            }
        };

        rows
    }

    fn generate_start_anchor(&mut self, anchor: usize, area: Rect) -> Vec<Vec<StyledGrapheme<'_>>> {
        let lines = std::mem::take(&mut self.text_lines);
        let mut current_ln = anchor;
        let mut rows = self.generate_rows_down(&mut current_ln, &lines, &area);
        if let Some(metadata_handler) = &mut self.metadata_handler {
            metadata_handler(anchor..current_ln);
        }

        rows
    }

    fn extract_ln_styling(
        map: &HashMap<TextCoord, tui::style::Style>,
        ln_offset: usize,
    ) -> HashMap<usize, tui::style::Style> {
        map.iter()
            .filter_map(|(coord, &style)| (coord.ln == ln_offset).then(|| (coord.x, style)))
            .collect()
    }

    fn line_to_rows<'txt>(
        &mut self,
        line_nb: usize,
        line: &[(&'txt str, tui::style::Style)],
        area: &Rect,
    ) -> Vec<Vec<StyledGrapheme<'txt>>> {
        let styling = Self::extract_ln_styling(&self.sparse_styling, line_nb);
        self.line_processor.process_line(line, styling, area.width)
    }

    fn generate_end_anchor(&mut self, anchor: usize, area: Rect) -> Vec<Vec<StyledGrapheme<'_>>> {
        let lines = std::mem::take(&mut self.text_lines);
        let mut start_ln = anchor - 1;
        let mut end_ln = anchor;

        let mut rows = self.generate_rows_up(&mut start_ln, &lines, &area);
        let mut bottom_rows = self.generate_rows_down(&mut end_ln, &lines, &area);
        rows.extend(bottom_rows);

        // passing the actually displayed line range
        if let Some(metadata_handler) = &mut self.metadata_handler {
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

        let lines = self.generate_view(area);
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
