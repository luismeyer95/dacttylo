use std::collections::HashMap;
use std::ops::Range;

use syntect::highlighting::Style;
use tui::{
    buffer::Buffer,
    layout::Rect,
    text::StyledGrapheme,
    widgets::{Block, StatefulWidget, Widget},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::highlight::{Highlighter, NoHighlight, SyntectHighlight};
use crate::utils;
use crate::utils::reflow::{LineComposer, WordWrapper};

pub struct TextView<'a> {
    /// The full text buffer
    text_lines: Vec<&'a str>,

    /// Generic syntax highlighter
    syntax_styling: Box<dyn Highlighter>,

    /// Sparse styling applied after the syntax highlight pass,
    /// used for cursors and special application logic highlighting
    sparse_styling: HashMap<usize, tui::style::Style>,

    /// Enclosing block component
    block: Block<'a>,

    /// Option to override the background color after all styles are applied
    bg_color: tui::style::Color,

    /// Optional closure to set external UI state from the wrapped view slice
    wrap_event_handler: Option<Box<dyn Fn(Vec<usize>, u16) + 'a>>,
}

impl<'a> TextView<'a> {
    const TAB_SYMBOL: &'static str = "\u{21e5}";
    const NL_SYMBOL: &'static str = "\u{23ce}";

    /// Instantiate a TextView widget from a line buffer and use the builder
    /// pattern to set custom rendering options
    pub fn new(text_lines: Vec<&'a str>) -> Self {
        Self {
            text_lines,
            syntax_styling: Box::new(SyntectHighlight),
            sparse_styling: HashMap::new(),
            block: Default::default(),
            bg_color: tui::style::Color::Reset,
            wrap_event_handler: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block;
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

    pub fn bg_color(mut self, color: tui::style::Color) -> Self {
        self.bg_color = color;
        self
    }

    /// Pass a closure to this function to set external UI state.
    /// The closure is passed
    /// - a vector of line heights (acts as a map from line number to row count)
    /// - the height of the text view render buffer
    pub fn on_wrap(mut self, closure: Box<dyn Fn(Vec<usize>, u16) + 'a>) -> Self {
        self.wrap_event_handler = Some(closure);
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
        // split text buffer by newline
        let lines = std::mem::take(&mut self.text_lines);
        let view_slice = &lines[0..area.height as usize];
        let view_slice = self.syntax_styling.highlight(view_slice);
        // apply text transforms and sparse styling
        let view_slice = self.apply_transforms(0, view_slice);

        // once transforms are done we can wrap the lines to the output area width
        let wrapped_slice = Self::wrap_lines(area.width, view_slice);

        // call the wrap handler for consumers to update UI state between renders
        if let Some(func) = self.wrap_event_handler.take() {
            func(wrapped_slice.iter().map(|v| v.len()).collect(), area.height);
        }

        wrapped_slice.into_iter().flat_map(|v| v).collect()
    }

    fn apply_transforms<'txt>(
        &mut self,
        mut key_offset: usize,
        lines: Vec<Vec<(&'txt str, tui::style::Color)>>,
    ) -> Vec<Vec<StyledGrapheme<'txt>>> {
        lines
            .into_iter()
            .map(|tkns_line| Self::tokens_to_graphemes(tkns_line.as_slice()))
            .map(|graphemes_line| {
                let transformed_line = self.transform_line(key_offset, &graphemes_line);
                key_offset += graphemes_line.iter().count();
                transformed_line
            })
            .collect()
    }

    fn tokens_to_graphemes<'tkn>(
        tokens: &[(&'tkn str, tui::style::Color)],
    ) -> Vec<StyledGrapheme<'tkn>> {
        tokens
            .into_iter()
            .flat_map(|(token, color)| {
                token.graphemes(true).map(|g| StyledGrapheme {
                    symbol: g,
                    style: tui::style::Style::default().fg(*color),
                })
            })
            .collect::<Vec<StyledGrapheme<'tkn>>>()
    }

    fn transform_line<'txt>(
        &self,
        accumulated_offset: usize,
        graphemes: &[StyledGrapheme<'txt>],
    ) -> Vec<StyledGrapheme<'txt>> {
        let mut key_offset = accumulated_offset;
        let mut inline_offset = 0;
        let mut transformed_line: Vec<StyledGrapheme> = vec![];
        transformed_line.push(StyledGrapheme {
            symbol: "~ ",
            style: tui::style::Style::default(),
        });

        for gphm in graphemes.into_iter() {
            let remapped_key = Self::remap_symbol(gphm.clone(), inline_offset);
            let styled_key = self.apply_sparse_styling(key_offset, remapped_key);
            let size = styled_key.iter().count();

            transformed_line.extend(styled_key);

            key_offset += 1;
            inline_offset += size;
        }
        transformed_line
    }

    fn remap_symbol<'txt>(
        grapheme: StyledGrapheme<'txt>,
        inline_index: usize,
    ) -> Vec<StyledGrapheme<'txt>> {
        match grapheme.symbol {
            "\n" => Self::remap_newline(grapheme),
            "\t" => Self::remap_tab(grapheme, inline_index),
            _ => vec![grapheme],
        }
    }

    fn remap_tab(grapheme: StyledGrapheme, inline_index: usize) -> Vec<StyledGrapheme> {
        let tab_width = (4 - inline_index % 4) as u8;
        let style = grapheme
            .style
            .patch(tui::style::Style::default().fg(tui::style::Color::Yellow));

        vec![StyledGrapheme {
            symbol: Self::TAB_SYMBOL,
            style,
        }]
        .into_iter()
        .chain(vec![
            StyledGrapheme { symbol: " ", style };
            (tab_width - 1) as usize
        ])
        .collect()
    }

    fn remap_newline(grapheme: StyledGrapheme) -> Vec<StyledGrapheme> {
        vec![
            StyledGrapheme {
                symbol: Self::NL_SYMBOL,
                style: grapheme
                    .style
                    .patch(tui::style::Style::default().fg(tui::style::Color::Yellow)),
            },
            grapheme,
        ]
    }

    fn apply_sparse_styling<'txt>(
        &self,
        key_offset: usize,
        mut key_as_graphemes: Vec<StyledGrapheme<'txt>>,
    ) -> Vec<StyledGrapheme<'txt>> {
        self.sparse_styling
            .get(&key_offset)
            .map(|style| key_as_graphemes[0].style = *style);
        key_as_graphemes
    }

    fn wrap_lines(width: u16, lines: Vec<Vec<StyledGrapheme>>) -> Vec<Vec<Vec<StyledGrapheme>>> {
        lines
            .into_iter()
            .map(|line| Self::wrap_line(width, line))
            .collect()
    }

    fn wrap_line(width: u16, graphemes: Vec<StyledGrapheme>) -> Vec<Vec<StyledGrapheme>> {
        let mut graphemes_it = graphemes.into_iter();
        let mut line_composer = WordWrapper::new(&mut graphemes_it, width, false);
        let mut lines: Vec<Vec<StyledGrapheme>> = vec![];

        while let Some((current_line, _)) = line_composer.next_line() {
            lines.push(current_line.into_iter().cloned().collect());
        }

        lines
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

pub struct RenderMetadata {
    line_rowcount_map: Vec<usize>,
    buffer_height: u16,
}

pub struct EditorView<'a> {
    pub text_lines: Vec<&'a str>,
    pub view_anchor: usize,
    pub tracked_line: usize,

    pub last_render: RenderMetadata,
}

impl<'a> StatefulWidget for TextView<'a> {
    type State = EditorView<'a>;

    fn render(mut self, mut area: Rect, buf: &mut Buffer, state: &mut Self::State) {
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
