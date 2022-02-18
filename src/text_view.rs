use crate::line::processor::LineProcessor;
use crate::line::stylizer::LineStylizer;
use crate::utils::types::StyledLine;
use std::cmp::min;
use std::cmp::Ordering;
use tui::style::{Color, Style};
use tui::{
    buffer::Buffer,
    layout::Rect,
    text::StyledGrapheme,
    widgets::{Block, Widget},
};
use unicode_width::UnicodeWidthStr;

pub enum Anchor {
    Start(usize),
    Center(usize),
    End(usize),
}

/// Lower level, stateless text displaying engine.
pub struct TextView<'a, 'ln> {
    /// The full text buffer
    text_lines: &'ln [StyledLine<'a>],

    /// Controls the line offset behaviour for the final display
    anchor: Anchor,

    /// Responsible for transforming a line to a collection of rows
    /// given a terminal width size
    line_processor: Box<dyn LineProcessor>,

    /// Enclosing block component
    block: Block<'a>,

    /// Option to override the background color after all styles are applied
    bg_color: Option<Color>,
}

impl<'a, 'ln> TextView<'a, 'ln> {
    /// Instantiate a TextView widget from a line buffer and use the builder
    /// pattern to set custom rendering options

    pub fn from_styled_content(lines: &'ln [StyledLine<'a>]) -> Self {
        Self {
            text_lines: lines,
            line_processor: Box::new(LineStylizer),
            anchor: Anchor::Start(0),
            block: Default::default(),
            bg_color: None,
        }
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
        self.anchor = match anchor {
            Anchor::Start(anchor) if anchor >= self.text_lines.len() => {
                panic!("Anchor out of bounds")
            }
            Anchor::End(anchor) => {
                Anchor::End(min(anchor, self.text_lines.len()))
            }
            _ => anchor,
        };
        self
    }

    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = Some(color);
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
            Anchor::Center(anchor) => self.generate_center_anchor(anchor, area),
            _ => panic!("Disabled anchors"),
        }
    }

    fn generate_center_anchor(
        &mut self,
        anchor: usize,
        area: Rect,
    ) -> Vec<Vec<StyledGrapheme<'_>>> {
        let half_height_area = Rect::new(0, 0, area.width, area.height / 2);

        let (_, mut rows) = self.expand_rows_up(anchor, half_height_area);

        let area = Rect::new(0, 0, area.width, area.height - rows.len() as u16); // cast should be safe
        let (_, bottom_rows) = self.expand_rows_down(anchor + 1, area);
        rows.extend(bottom_rows);

        rows
    }

    /// Generates rows downwards and returns the line nb past the last rendered line along with the rows
    fn expand_rows_down(
        &self,
        start_ln: usize,
        area: Rect,
    ) -> (usize, Vec<Vec<StyledGrapheme<'_>>>) {
        let total_lines = self.text_lines.len();
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

    fn line_to_rows(
        &self,
        line_nb: usize,
        width: u16,
    ) -> Vec<Vec<StyledGrapheme<'_>>> {
        let line = self.text_lines[line_nb].as_slice();
        let mut graphemes = line.to_owned().into_iter();
        let bg = self.bg_color.unwrap_or(Color::Reset);

        self.line_processor.process_line(&mut graphemes, width, bg)
    }
}

impl<'a, 'ln> Widget for TextView<'a, 'ln> {
    fn render(mut self, mut area: Rect, buf: &mut Buffer) {
        self.render_block(&mut area, buf);
        if area.height < 1 || area.width < 1 {
            return;
        }

        let bg_style =
            Style::default().bg(self.bg_color.unwrap_or(Color::Reset));
        buf.set_style(area, bg_style);

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
