use crossterm::cursor;
use itertools::Itertools;
use once_cell::sync::OnceCell;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::env::current_dir;
use std::error::Error;
use std::io::BufRead;
use std::iter::Peekable;
use std::ops::{Not, Range};
use std::str::CharIndices;
use std::str::FromStr;
use syntect::easy::{HighlightFile, HighlightLines};
use syntect::highlighting::{self, FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

use std::iter;
use tui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{StyledGrapheme, Text},
    widgets::{
        // reflow::{LineComposer, LineTruncator, WordWrapper},
        Block,
        Widget,
    },
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// use std::borrow::Cow;

fn input_width(s: &str) -> usize {
    UnicodeSegmentation::graphemes(s, true).count()
}

/// An implementation of this trait will be given to a Typeview object
/// on instantiation
pub trait Highlighter {
    fn highlight(&self, text: &str) -> Vec<(&str, tui::style::Style)>;
}

const TAB_SYMBOL: &str = "\u{21e5}";
const NL_SYMBOL: &str = "\u{23ce}";

struct TypeView<'a> {
    /// The full text buffer
    text: &'a str,

    /// Grapheme index around which the view should be vertically centered
    context_pos: usize,

    /// Generic syntax highlighter
    syntax_styling: Box<dyn Highlighter>,

    /// Sparse styling applied after the syntax highlight pass.
    /// Used for cursors and special application logic highlighting
    sparse_styling: Vec<(usize, tui::style::Style)>,
}

impl<'a> TypeView<'a> {
    pub fn new(
        text: &'a str,
        context_pos: usize,
        syntax_styling: Box<dyn Highlighter>,
        sparse_styling: Vec<(usize, tui::style::Style)>,
    ) -> Self {
        Self {
            text,
            context_pos,
            syntax_styling,
            sparse_styling,
        }
    }

    /// Converts a 1D text buffer position into a tuple containing
    /// line number and a character index into that line
    fn text_to_line_index(
        index: usize,
        text_lines: &Vec<&str>,
    ) -> Result<(usize, usize), &'static str> {
        let mut cur_index = index;
        for (i, &line) in text_lines.iter().enumerate() {
            let w = input_width(&line);
            if (w > cur_index) {
                return Ok((i, cur_index));
            }
            cur_index -= w;
        }
        Err("index out of bounds")
    }

    fn line_to_text_index(ln_index: usize, text_lines: Vec<&str>) -> Result<usize, &'static str> {
        if ln_index > text_lines.len() {
            Err("index out of bounds")
        } else {
            Ok(text_lines
                .into_iter()
                .enumerate()
                .take_while(|(i, el)| i != &ln_index)
                .fold(0, |acc, (i, el)| acc + input_width(&el)))
        }
    }

    fn filter_viewable_sparse_styling(
        &mut self,
        start: usize,
        end: usize,
    ) -> Vec<(usize, tui::style::Style)> {
        self.sparse_styling
            .iter()
            .cloned()
            .filter(|(i, _)| i >= &start && i < &end)
            .collect::<Vec<_>>()
    }

    fn get_view<'lines>(
        &self,
        view_height: usize,
        text_lines: &'lines Vec<&'a str>,
    ) -> &'lines [&str] {
        let (ctx_line_nb, _) = Self::text_to_line_index(self.context_pos, &text_lines).unwrap();

        let lower_bound = ctx_line_nb.saturating_sub(view_height / 2);
        let upper_bound = std::cmp::min(
            text_lines.len(),
            ctx_line_nb.saturating_add(view_height / 2),
        );

        &text_lines[lower_bound..upper_bound]
    }

    fn offset_sparse_stylings(&mut self, offset_index: usize, len: usize) {
        for (styled_idx, style) in &mut self.sparse_styling {
            if *styled_idx > offset_index {
                *styled_idx += len - 1;
            }
        }
    }

    fn transform_symbols(
        &mut self,
        view_line_offset: usize,
        hl_lines: Vec<(&'a str, tui::style::Style)>,
    ) -> Vec<StyledGrapheme> {
        let graphemes = Self::styled_graphemes(&hl_lines);

        let line_pos_it = hl_lines.iter().flat_map(|(tkn, _)| {
            tkn.graphemes(true)
                .enumerate()
                .map(|(i, _)| i)
                .collect::<Vec<usize>>()
        });

        graphemes
            .into_iter()
            .zip(line_pos_it)
            .enumerate()
            .flat_map(|(idx, (gphm, idx_in_line))| match gphm.symbol {
                "\n" => vec![StyledGrapheme {
                    symbol: NL_SYMBOL,
                    style: gphm
                        .style
                        .patch(tui::style::Style::default().fg(tui::style::Color::Yellow)),
                }],
                "\t" => {
                    let tab_width = (4 - idx_in_line % 4) as u8;
                    self.offset_sparse_stylings(idx, tab_width as usize);
                    let style = gphm
                        .style
                        .patch(tui::style::Style::default().fg(tui::style::Color::Yellow));
                    vec![StyledGrapheme {
                        symbol: TAB_SYMBOL,
                        style,
                    }]
                    .into_iter()
                    .chain(vec![
                        StyledGrapheme { symbol: " ", style };
                        (tab_width - 1) as usize
                    ])
                    .collect()
                }
                _ => vec![gphm],
            })
            .collect()
    }

    fn styled_graphemes<'tkn>(
        text: &[(&'tkn str, tui::style::Style)],
    ) -> Vec<StyledGrapheme<'tkn>> {
        text.into_iter()
            .flat_map(|(token, style)| {
                token.graphemes(true).map(|g| StyledGrapheme {
                    symbol: g,
                    style: style.clone(),
                })
            })
            .collect()
    }
}

impl<'a> Widget for TypeView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // get the output area height to determine the minimum
        // number of lines needed for the view
        let view_linecount = area.height as usize;

        // extract the necessary lines from around the context position
        let text_lines: Vec<&str> = self.text.split_inclusive('\n').collect();
        let view_slice = self.get_view(view_linecount, &text_lines);

        let view_slice = view_slice
            .into_iter()
            .map(|s| s.chars())
            .flatten()
            .collect::<String>();

        //apply highlighting
        let highlighted_lines = self.syntax_styling.highlight(&view_slice);

        let transformed_lines = self.transform_symbols(highlighted_lines);
    }
}
