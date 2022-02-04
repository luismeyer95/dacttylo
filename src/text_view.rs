use crate::{
    line_processor::LineProcessor, line_stylizer::LineStylizer,
    text_coord::TextCoord,
};
use std::cell::RefCell;
use std::cmp::min;
use std::ops::{Deref, Range};
use std::{cmp::Ordering, collections::HashMap};
use tui::{
    buffer::Buffer,
    layout::Rect,
    text::StyledGrapheme,
    widgets::{Block, Widget},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use tui::widgets::{Paragraph, StatefulWidget};

// type StyledLine<'a> = Vec<(&'a str, tui::style::Style)>;
type StyledLineIterator<'a> = Box<dyn Iterator<Item = StyledGrapheme<'a>> + 'a>;

#[derive(Debug, Clone)]
pub enum Anchor {
    Start(usize),
    Center(usize),
    End(usize),
}

#[derive(Debug, Clone)]
pub struct RenderMetadata {
    pub lines_rendered: Range<usize>,
    pub anchor: Anchor,
}

/// Lower level, stateless text displaying engine.
pub struct TextView<'a> {
    /// The full text buffer
    text_lines: RefCell<Vec<StyledLineIterator<'a>>>,

    /// Controls the line offset behaviour for the final display
    anchor: Anchor,

    /// Responsible for transforming a line to a collection of rows
    /// given a terminal width size
    line_processor: Box<dyn LineProcessor>,

    /// Styling applied after the syntax highlight pass,
    /// used for cursors and special application logic highlighting
    sparse_styling: HashMap<TextCoord, tui::style::Style>,

    /// Enclosing block component
    block: Block<'a>,

    /// Option to override the background color after all styles are applied
    bg_color: tui::style::Color,

    cursor: Option<TextCoord>,
}

impl<'a> TextView<'a> {
    /// Instantiate a TextView widget from a line buffer and use the builder
    /// pattern to set custom rendering options
    pub fn new() -> Self {
        Self {
            text_lines: vec![].into(),
            line_processor: Box::new(LineStylizer),
            anchor: Anchor::Start(0),
            sparse_styling: HashMap::new(),
            block: Default::default(),
            bg_color: tui::style::Color::Reset,
            cursor: None,
        }
    }

    pub fn content<Lns, Ref>(mut self, lines: Lns) -> Self
    where
        Lns: IntoIterator<Item = Ref>,
        Ref: Deref<Target = &'a str>,
    {
        self.text_lines = lines
            .into_iter()
            .map(|s| {
                Box::new(s.graphemes(true).map(|g| StyledGrapheme {
                    symbol: g,
                    style: tui::style::Style::default(),
                })) as Box<dyn Iterator<Item = StyledGrapheme>>
            })
            .collect::<Vec<_>>()
            .into();
        self
    }

    pub fn styled_content<Lns, Ln>(mut self, lines: Lns) -> Self
    where
        Lns: Iterator<Item = Ln>,
        Ln: Into<Box<dyn Iterator<Item = StyledGrapheme<'a>> + 'a>>,
    {
        self.text_lines = lines
            .into_iter()
            .map(|s| s.into() as Box<dyn Iterator<Item = StyledGrapheme>>)
            .collect::<Vec<_>>()
            .into();
        self
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block;
        self
    }

    pub fn line_processor(
        mut self,
        line_processor: Box<dyn LineProcessor>,
    ) -> Self {
        self.line_processor = line_processor;
        self
    }

    pub fn anchor(mut self, anchor: Anchor) -> Self {
        let line_count = self.text_lines.borrow().len();

        self.anchor = match anchor {
            Anchor::Start(anchor) if anchor >= line_count => {
                panic!("Anchor out of bounds")
            }
            Anchor::End(anchor) => Anchor::End(min(anchor, line_count)),
            _ => anchor,
        };
        self
    }

    // pub fn cursor(coord: TextCoord) {}

    pub fn sparse_styling(
        mut self,
        sparse_styling: HashMap<TextCoord, tui::style::Style>,
    ) -> Self {
        self.sparse_styling = sparse_styling;
        self
    }

    pub fn bg_color(mut self, color: tui::style::Color) -> Self {
        self.bg_color = color;
        self
    }

    fn render_block(&mut self, area: &mut Rect, buf: &mut Buffer) {
        let block = std::mem::take(&mut self.block);

        // save the inner_area because render consumes the block
        let inner_area = block.inner(*area);
        block.render(*area, buf);

        *area = inner_area;
    }

    fn generate_view(
        &mut self,
        area: Rect,
        meta: &mut Option<RenderMetadata>,
    ) -> Vec<Vec<StyledGrapheme<'_>>> {
        match self.anchor {
            Anchor::Start(anchor) => {
                self.generate_start_anchor(anchor, area, meta)
            }
            Anchor::End(anchor) => self.generate_end_anchor(anchor, area, meta),
            Anchor::Center(anchor) => {
                self.generate_center_anchor(anchor, area, meta)
            }
            _ => panic!("Disabled anchors"),
        }
    }

    fn generate_start_anchor(
        &mut self,
        anchor: usize,
        area: Rect,
        meta: &mut Option<RenderMetadata>,
    ) -> Vec<Vec<StyledGrapheme<'_>>> {
        let (end_ln, rows) = self.expand_rows_down(anchor, area);

        *meta = Some(RenderMetadata {
            lines_rendered: anchor..end_ln,
            anchor: Anchor::Start(anchor),
        });

        rows
    }

    fn generate_end_anchor(
        &mut self,
        anchor: usize,
        area: Rect,
        meta: &mut Option<RenderMetadata>,
    ) -> Vec<Vec<StyledGrapheme<'_>>> {
        let (start_ln, mut rows) = self.expand_rows_up(anchor - 1, area);
        let area = Rect::new(0, 0, area.width, area.height - rows.len() as u16); // cast should be safe
        let (end_ln, bottom_rows) = self.expand_rows_down(anchor, area);
        rows.extend(bottom_rows);

        // passing the actually displayed line range
        *meta = Some(RenderMetadata {
            lines_rendered: start_ln..end_ln,
            anchor: Anchor::End(anchor),
        });

        rows
    }

    fn generate_center_anchor(
        &mut self,
        anchor: usize,
        area: Rect,
        meta: &mut Option<RenderMetadata>,
    ) -> Vec<Vec<StyledGrapheme<'_>>> {
        let half_height_area = Rect::new(0, 0, area.width, area.height / 2);

        let (start_ln, mut rows) =
            self.expand_rows_up(anchor, half_height_area);

        let area = Rect::new(0, 0, area.width, area.height - rows.len() as u16); // cast should be safe
        let (end_ln, bottom_rows) = self.expand_rows_down(anchor + 1, area);
        rows.extend(bottom_rows);

        // passing the actually displayed line range
        *meta = Some(RenderMetadata {
            lines_rendered: start_ln..end_ln,
            anchor: Anchor::Center(anchor),
        });

        rows
    }

    /// Generates rows downwards and returns the line nb past the last rendered line along with the rows
    fn expand_rows_down(
        &self,
        start_ln: usize,
        area: Rect,
    ) -> (usize, Vec<Vec<StyledGrapheme<'_>>>) {
        let total_lines = self.text_lines.borrow().len();
        let mut rows: Vec<Vec<StyledGrapheme<'_>>> = vec![];

        for current_ln in start_ln..total_lines {
            let line_as_rows = self.line_to_rows(current_ln, area.width);
            rows.extend(line_as_rows);
            if rows.len() > area.height.into() {
                rows.truncate(area.height.into());
                return (current_ln, rows);
            }
        }

        (total_lines, rows)
    }

    /// Generates rows upwards and returns the lowest line nb to be rendered along with the rows
    fn expand_rows_up(
        &self,
        start_ln: usize,
        area: Rect,
    ) -> (usize, Vec<Vec<StyledGrapheme<'_>>>) {
        let mut rows: Vec<Vec<StyledGrapheme<'_>>> = vec![];

        for current_ln in (0..=start_ln).rev() {
            let mut line_as_rows = self.line_to_rows(current_ln, area.width);
            // if line_as_rows.len() + rows.len() > area.height as usize {
            //     return (current_ln + 1, rows);
            // }
            line_as_rows.extend(rows);
            rows = line_as_rows;

            match rows.len().cmp(&(area.height as usize)) {
                Ordering::Equal => return (current_ln, rows),
                Ordering::Greater => {
                    rows.drain(0..(rows.len() - area.height as usize));
                    return (current_ln + 1, rows);
                }
                _ => {}
            }
        }

        (0, rows)
    }

    fn extract_ln_styling(
        map: &HashMap<TextCoord, tui::style::Style>,
        ln_offset: usize,
    ) -> HashMap<usize, tui::style::Style> {
        map.iter()
            .filter_map(|(coord, &style)| {
                (coord.ln == ln_offset).then(|| (coord.x, style))
            })
            .collect()
    }

    fn line_to_rows(
        &self,
        line_nb: usize,
        width: u16,
    ) -> Vec<Vec<StyledGrapheme<'_>>> {
        let mut line = &mut self.text_lines.borrow_mut()[line_nb];
        let styling = Self::extract_ln_styling(&self.sparse_styling, line_nb);
        self.line_processor.process_line(line, styling, width)
    }
}

impl<'a> Default for TextView<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> StatefulWidget for TextView<'a> {
    type State = Option<RenderMetadata>;

    fn render(
        mut self,
        mut area: Rect,
        buf: &mut Buffer,
        state: &mut Self::State,
    ) {
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

        let lines = self.generate_view(area, state);
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
