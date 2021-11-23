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
use std::hash::Hash;
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
    widgets::{Block, Widget},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::reflow::{LineComposer, WordWrapper};

fn input_width(s: &str) -> usize {
    UnicodeSegmentation::graphemes(s, true).count()
}

/// Converts a 1D text buffer position into a tuple containing
/// line number and a character index into that line
fn text_to_line_index(
    index: usize,
    text_lines: &Vec<&str>,
) -> Result<(usize, usize), &'static str> {
    let mut offset = index;
    for (ln_index, &line) in text_lines.iter().enumerate() {
        let ln_width = input_width(&line);
        if (0..ln_width).contains(&offset) {
            return Ok((ln_index, offset));
        }
        offset -= ln_width;
    }
    Err("index out of bounds")
}

fn line_to_text_index(ln_index: usize, text_lines: &[&str]) -> Result<usize, &'static str> {
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

fn load_defaults() -> (&'static SyntaxSet, &'static ThemeSet) {
    static SYNTAX_SET: OnceCell<SyntaxSet> = OnceCell::new();
    static THEME_SET: OnceCell<ThemeSet> = OnceCell::new();
    (
        SYNTAX_SET.get_or_init(|| SyntaxSet::load_defaults_newlines()),
        THEME_SET.get_or_init(|| ThemeSet::load_defaults()),
    )
}

fn syntect_to_tui_style(syntect_style: syntect::highlighting::Style) -> tui::style::Style {
    type TuiStyle = tui::style::Style;
    type SyntectStyle = syntect::highlighting::Style;
    type SyntectMod = syntect::highlighting::FontStyle;
    type TuiMod = tui::style::Modifier;
    type TuiColor = tui::style::Color;

    let mut style = TuiStyle::default()
        .fg(tui::style::Color::Rgb(
            syntect_style.foreground.r,
            syntect_style.foreground.g,
            syntect_style.foreground.b,
        ))
        .bg(tui::style::Color::Rgb(
            syntect_style.background.r,
            syntect_style.background.g,
            syntect_style.background.b,
        ));
    if syntect_style.font_style.contains(SyntectMod::BOLD) {
        style = style.add_modifier(TuiMod::BOLD)
    }
    if syntect_style.font_style.contains(SyntectMod::UNDERLINE) {
        style = style.add_modifier(TuiMod::UNDERLINED)
    }
    if syntect_style.font_style.contains(SyntectMod::ITALIC) {
        style = style.add_modifier(TuiMod::ITALIC)
    }

    style
}

struct NoHighlight;
impl Highlighter for NoHighlight {
    fn highlight<'txt>(&self, text: &'txt str) -> Vec<(&'txt str, tui::style::Style)> {
        // vec![(text, Default::default())]
        let (syntax_set, theme_set) = load_defaults();
        let syntax = syntax_set
            .find_syntax_by_extension("rs")
            .expect("syntax extension not found");

        let mut highlighter = HighlightLines::new(syntax, &theme_set.themes["base16-ocean.dark"]);

        let mut tokenized_contents: Vec<(syntect::highlighting::Style, &str)> = vec![];
        for line in LinesWithEndings::from(&text) {
            let mut tokens: Vec<(syntect::highlighting::Style, &str)> =
                highlighter.highlight(&line, &syntax_set);
            tokenized_contents.extend(tokens);
        }

        tokenized_contents
            .into_iter()
            .map(|(style, token)| (token, syntect_to_tui_style(style)))
            .collect()
    }
}

const TAB_SYMBOL: &str = "\u{21e5}";
const NL_SYMBOL: &str = "\u{23ce}";

pub struct TypeView<'a> {
    /// The full text buffer
    text: &'a str,

    /// Grapheme index around which the view should be vertically centered
    context_pos: usize,

    /// Generic syntax highlighter
    syntax_styling: Box<dyn Highlighter>,

    /// Sparse styling applied after the syntax highlight pass.
    /// Used for cursors and special application logic highlighting
    sparse_styling: HashMap<usize, tui::style::Style>,

    block: Block<'a>,
}

impl<'a> TypeView<'a> {
    pub fn new(
        text: &'a str,
        // context_pos: usize,
        // syntax_styling: Box<dyn Highlighter>,
        // sparse_styling: HashMap<usize, tui::style::Style>,
    ) -> Self {
        Self {
            text,
            context_pos: 0,
            syntax_styling: Box::new(NoHighlight),
            sparse_styling: HashMap::new(),
            block: Default::default(),
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block;
        self
    }

    pub fn context_pos(mut self, context_pos: usize) -> Self {
        self.context_pos = context_pos;
        self
    }

    pub fn syntax_styling(mut self, syntax_styling: Box<dyn Highlighter>) -> Self {
        self.syntax_styling = syntax_styling;
        self
    }

    pub fn sparse_styling(mut self, sparse_styling: HashMap<usize, tui::style::Style>) -> Self {
        self.sparse_styling = sparse_styling;
        self
    }

    fn get_view_vertical_range<'lines>(
        &self,
        view_height: usize,
        text_lines: &'lines Vec<&'a str>,
    ) -> Range<usize> {
        let (ctx_line_nb, _) = text_to_line_index(self.context_pos, &text_lines).unwrap();
        let lower_bound = ctx_line_nb.saturating_sub(view_height / 2);
        let rest = view_height.saturating_sub(lower_bound);
        let upper_bound = std::cmp::min(text_lines.len(), ctx_line_nb.saturating_add(rest));
        lower_bound..upper_bound
    }

    fn remap_symbol<'txt>(
        inline_index: usize,
        grapheme: StyledGrapheme<'txt>,
    ) -> Vec<StyledGrapheme<'txt>> {
        match grapheme.symbol {
            "\n" => vec![
                StyledGrapheme {
                    symbol: NL_SYMBOL,
                    style: grapheme
                        .style
                        .patch(tui::style::Style::default().fg(tui::style::Color::Yellow)),
                },
                grapheme,
            ],
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

    fn wrap_lines(width: u16, graphemes: Vec<StyledGrapheme>) -> Vec<Vec<StyledGrapheme>> {
        let mut graphemes_it = graphemes.into_iter();

        let mut line_composer: Box<dyn LineComposer> =
            Box::new(WordWrapper::new(&mut graphemes_it, width, false));

        let mut lines: Vec<Vec<StyledGrapheme>> = vec![];
        while let Some((current_line, _)) = line_composer.next_line() {
            lines.push(current_line.into_iter().cloned().collect());
        }
        lines
    }
}

impl<'a> Widget for TypeView<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        let block = std::mem::take(&mut self.block);
        let inner_area = block.inner(area);
        block.render(area, buf);
        let area = inner_area;
        if area.height < 1 || area.width < 1 {
            return;
        }
        // get the output area height to determine the minimum
        // number of lines needed for the view
        let view_height = area.height as usize;

        // extract the necessary lines from around the context position (view slice)
        let text_lines: Vec<&str> = self.text.split_inclusive('\n').collect();
        let view_range = self.get_view_vertical_range(view_height, &text_lines);
        let view_slice = &text_lines[view_range.clone()];

        // saving the total text offset (index of the first character inside the view range)
        let total_offset = line_to_text_index(view_range.start, &text_lines).unwrap();

        //
        let view_slice = view_slice
            .into_iter()
            .flat_map(|s| s.graphemes(true))
            .collect::<String>();

        // apply highlighting, obtain styled tokens
        let highlighted_tokens = self.syntax_styling.highlight(&view_slice);
        let graphemes = self.transform_to_view(total_offset, highlighted_tokens);
        let wrapped_lines = Self::wrap_lines(area.width as u16, graphemes);

        if wrapped_lines.len() != 0 && wrapped_lines[0].len() != 0 {
            buf.set_style(area, wrapped_lines[0][0].style);
        }
        let mut y = 0;
        for line in wrapped_lines {
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
