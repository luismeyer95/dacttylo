use crossterm::cursor;
use itertools::Itertools;
use once_cell::sync::OnceCell;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::hash_map::Entry::Occupied;
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
        reflow::{LineComposer, LineTruncator, WordWrapper},
        Block, Widget,
    },
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// use std::borrow::Cow;

fn input_width(s: &str) -> usize {
    UnicodeSegmentation::graphemes(s, true).count()
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

fn slice_diff<T>(a: &[T], b: &[T]) -> usize {
    b.as_ptr() as usize - a.as_ptr() as usize
}

fn styled_graphemes<'tkn>(text: &[(&'tkn str, tui::style::Style)]) -> Vec<StyledGrapheme<'tkn>> {
    text.into_iter()
        .flat_map(|(token, style)| {
            token.graphemes(true).map(|g| StyledGrapheme {
                symbol: g,
                style: style.clone(),
            })
        })
        .collect()
}

/// An implementation of this trait will be given to a Typeview object
/// on instantiation
pub trait Highlighter {
    fn highlight<'txt>(&self, text: &'txt str) -> Vec<(&'txt str, tui::style::Style)>;
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
    sparse_styling: HashMap<usize, tui::style::Style>,
}

impl<'a> TypeView<'a> {
    pub fn new(
        text: &'a str,
        context_pos: usize,
        syntax_styling: Box<dyn Highlighter>,
        sparse_styling: HashMap<usize, tui::style::Style>,
    ) -> Self {
        Self {
            text,
            context_pos,
            syntax_styling,
            sparse_styling,
        }
    }

    fn get_view<'lines>(
        &self,
        view_height: usize,
        text_lines: &'lines Vec<&'a str>,
    ) -> &'lines [&str] {
        let (ctx_line_nb, _) = text_to_line_index(self.context_pos, &text_lines).unwrap();

        let lower_bound = ctx_line_nb.saturating_sub(view_height / 2);
        let upper_bound = std::cmp::min(
            text_lines.len(),
            ctx_line_nb.saturating_add(view_height / 2),
        );

        &text_lines[lower_bound..upper_bound]
    }

    fn remap_symbol<'txt>(
        inline_index: usize,
        grapheme: StyledGrapheme<'txt>,
    ) -> Vec<StyledGrapheme<'txt>> {
        match grapheme.symbol {
            "\n" => vec![StyledGrapheme {
                symbol: NL_SYMBOL,
                style: grapheme
                    .style
                    .patch(tui::style::Style::default().fg(tui::style::Color::Yellow)),
            }],
            "\t" => {
                let tab_width = (4 - inline_index % 4) as u8;
                let style = grapheme
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
            _ => vec![grapheme],
        }
    }

    fn create_key_graphemes_map(lines: Vec<(&str, tui::style::Style)>) -> Vec<Vec<StyledGrapheme>> {
        let graphemes = styled_graphemes(&lines);

        let inline_index_it = lines.iter().flat_map(|(tkn, _)| {
            tkn.graphemes(true)
                .enumerate()
                .map(|(i, _)| i)
                .collect::<Vec<usize>>()
        });

        itertools::multizip((inline_index_it, graphemes))
            .map(|(inline_index, gphm)| Self::remap_symbol(inline_index, gphm))
            .collect::<Vec<Vec<StyledGrapheme>>>()
    }

    fn apply_sparse_styling<'txt>(
        &self,
        mapped_graphemes_it: impl Iterator<Item = (usize, Vec<StyledGrapheme<'txt>>)>,
    ) -> Vec<StyledGrapheme<'txt>> {
        mapped_graphemes_it
            .flat_map(|(i, mut key_as_graphemes)| {
                self.sparse_styling
                    .get(&i)
                    .map(|style| key_as_graphemes[0].style = *style);
                key_as_graphemes
            })
            .collect()
    }

    fn transform_to_view<'txt>(
        &mut self,
        total_offset: usize,
        lines: Vec<(&'txt str, tui::style::Style)>,
    ) -> Vec<StyledGrapheme<'txt>> {
        let mapped_graphemes_it = Self::create_key_graphemes_map(lines)
            .into_iter()
            .enumerate()
            .map(|(i, key_as_graphemes)| (i + total_offset, key_as_graphemes));

        self.apply_sparse_styling(mapped_graphemes_it)
    }
}

impl<'a> Widget for TypeView<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        // get the output area height to determine the minimum
        // number of lines needed for the view
        let view_linecount = area.height as usize;

        // extract the necessary lines from around the context position
        let text_lines: Vec<&str> = self.text.split_inclusive('\n').collect();
        let view_slice = self.get_view(view_linecount, &text_lines);
        let diff = slice_diff(&view_slice, &text_lines);

        let view_slice = view_slice
            .into_iter()
            .flat_map(|s| s.chars())
            .collect::<String>();

        //apply highlighting
        // let syntax_styling = std::mem::take(&mut self.syntax_styling).unwrap();
        let highlighted_lines = self.syntax_styling.highlight(&view_slice);

        let transformed_lines = self.transform_to_view(diff, highlighted_lines);
    }
}
