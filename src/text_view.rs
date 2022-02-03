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

const TAB_SYMBOL: &str = "\u{21e5}";
const NL_SYMBOL: &str = "\u{23ce}";
const SPACE: &str = " ";
const EMPTY: &str = "";

// type StyledLine<'a> = Vec<(&'a str, tui::style::Style)>;
type StyledLineIterator<'a> = Box<dyn Iterator<Item = StyledGrapheme<'a>> + 'a>;

#[derive(Debug, Clone)]
pub struct RenderMetadata {
    pub lines_rendered: Range<usize>,
    pub anchor: usize,
}

/// Lower level, stateless text displaying engine.
pub struct TextView<'a> {
    /// The full text buffer
    text_lines: RefCell<Vec<StyledLineIterator<'a>>>,

    /// Controls the line offset behaviour for the final display
    anchor: usize,

    /// Enclosing block component
    block: Block<'a>,

    /// Option to override the background color after all styles are applied
    bg_color: tui::style::Color,

    cursor: Option<TextCoord>,

    newline: StyledGrapheme<'static>,
    tab: StyledGrapheme<'static>,
}

impl<'a> TextView<'a> {
    /// Instantiate a TextView widget from a line buffer and use the builder
    /// pattern to set custom rendering options
    pub fn new() -> Self {
        Self {
            text_lines: vec![].into(),
            anchor: 0,
            block: Default::default(),
            bg_color: tui::style::Color::Reset,
            cursor: None,

            newline: StyledGrapheme {
                symbol: NL_SYMBOL,
                style: Default::default(),
            },
            tab: StyledGrapheme {
                symbol: TAB_SYMBOL,
                style: Default::default(),
            },
        }
    }

    pub fn content<Lns>(mut self, lines: Lns) -> Self
    where
        Lns: Iterator<Item = &'a str>,
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

    pub fn newline(mut self, cell: StyledGrapheme<'static>) -> Self {
        self.newline = cell;
        self
    }

    pub fn tab(mut self, cell: StyledGrapheme<'static>) -> Self {
        self.tab = cell;
        self
    }

    pub fn anchor(mut self, anchor: usize) -> Self {
        let line_count = self.text_lines.borrow().len();
        if anchor >= line_count {
            panic!("Anchor out of bounds")
        }
        self.anchor = anchor;
        self
    }

    // pub fn cursor(coord: TextCoord) {}

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
        buf.set_style(area.clone(), bg_style);

        let width = area.width as usize;
        let lines = self.text_lines.borrow_mut();

        let mut row_count = 0;
        let mut ln_count = 0;

        let max_drift = (area.width * area.height) as usize;
        loop {
            let (mut x, mut y): (u16, u16);
            let mut drift = 0;
            let set_drift = |sym: &str| {
                let sym_width = sym.width();
                if (drift + sym_width) % width < drift % width {
                    drift += (width - drift % width);
                }
                x = (drift % width) as u16;
                y = (drift / width) as u16;
            };

            for StyledGrapheme { symbol, style } in
                lines[self.anchor + ln_count]
            {
                if drift >= max_drift {
                    break;
                }
                set_drift(symbol);
                match symbol {
                    "\n" => {
                        buf.get_mut(
                            area.left() + x,
                            area.top() + row_count + y,
                        )
                        .set_symbol(self.newline.symbol)
                        .set_style(self.newline.style);
                        drift += self.newline.symbol.width();
                    }
                    "\t" => {
                        buf.get_mut(
                            area.left() + x,
                            area.top() + row_count + y,
                        )
                        .set_symbol(self.newline.symbol)
                        .set_style(self.newline.style);

                        drift += self.tab.symbol.width();

                        let padding = (4 - drift % 4);
                        for i in 0..padding {
                            set_drift(self.tab.symbol);

                            buf.get_mut(
                                area.left() + x,
                                area.top() + row_count + y,
                            )
                            .set_symbol(self.newline.symbol)
                            .set_style(self.newline.style);
                        }
                    }
                    _ => {
                        buf.get_mut(
                            area.left() + x,
                            area.top() + row_count + y,
                        )
                        .set_symbol(symbol)
                        .set_style(style);
                        drift += symbol.width();
                    }
                }
            }
            ln_count += 1;
            row_count += y + 1;
            if row_count >= area.height {
                break;
            }
        }

        // *meta = Some(RenderMetadata {
        //     lines_rendered: anchor..end_ln,
        //     anchor: Anchor::Start(anchor),
        // });
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::types::Coord;

    use super::*;
    use tui::{backend::TestBackend, buffer::Buffer, widgets::Block, Terminal};

    fn create_term(width: u16, height: u16) -> Terminal<TestBackend> {
        let backend = TestBackend::new(width, height);
        Terminal::new(backend).unwrap()
    }

    fn draw(
        terminal: &mut Terminal<TestBackend>,
        text_view: TextView,
        state: &mut Option<RenderMetadata>,
    ) {
        terminal
            .draw(|f| {
                f.render_stateful_widget(text_view, f.size(), state);
            })
            .unwrap();
    }

    #[test]
    fn anchor_start() {
        #[rustfmt::skip]
        let lines = [
            "Hello world!\n",
            "How are you?"
        ];

        let text_view =
            TextView::new().content(&lines).anchor(Anchor::Start(0));
        let mut metadata: Option<RenderMetadata> = None;

        let mut terminal = create_term(7, 4);
        draw(&mut terminal, text_view, &mut metadata);

        #[rustfmt::skip]
        let expected = Buffer::with_lines(vec![
            "Hello w",
            "orld!  ",
            "How are",
            " you?  ",
        ]);

        terminal.backend().assert_buffer(&expected);
        assert_eq!(
            metadata.unwrap(),
            RenderMetadata {
                lines_rendered: 0..2,
                anchor: Anchor::Start(0),
                cursor: None,
            }
        );
    }

    #[test]
    fn anchor_start_line_overflow() {
        #[rustfmt::skip]
        let lines = [
            "Hello world!\n",
            "How are you?"
        ];

        let text_view =
            TextView::new().content(&lines).anchor(Anchor::Start(0));
        let mut metadata: Option<RenderMetadata> = None;

        let mut terminal = create_term(4, 4);
        draw(&mut terminal, text_view, &mut metadata);

        #[rustfmt::skip]
        let expected = Buffer::with_lines(vec![
            "Hell",
            "o wo",
            "rld!",
            "How "
        ]);

        terminal.backend().assert_buffer(&expected);
        assert_eq!(
            metadata.unwrap(),
            RenderMetadata {
                lines_rendered: 0..1,
                anchor: Anchor::Start(0),
                cursor: None
            }
        );
    }
}
