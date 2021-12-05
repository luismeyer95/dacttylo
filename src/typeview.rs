use std::collections::HashMap;
use std::ops::Range;

use syntect::highlighting::Style;
use tui::{
    buffer::Buffer,
    layout::Rect,
    text::StyledGrapheme,
    widgets::{Block, Widget},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::highlight::{Highlighter, NoHighlight, SyntectHighlight};
use crate::utils;
use crate::utils::reflow::{LineComposer, WordWrapper};

struct FocusView<'l> {
    rows: Vec<&'l str>,
    context_pos: usize,
}

impl<'l> FocusView<'l> {
    pub fn new() -> Self {
        Self {
            rows: vec![],
            context_pos: 0,
        }
    }

    pub fn rows<T: AsRef<str>>(mut self, rows: &'l [T]) -> Self {
        self.rows = rows.into_iter().map(|x| x.as_ref()).collect();
        self
    }

    pub fn context_pos(mut self, context_pos: usize) -> Self {
        self.context_pos = context_pos;
        self
    }

    pub fn get_view_line_range(&self, view_height: usize) -> Range<usize> {
        // extract the minimum required lines from around the context position (view slice)
        let (context_line, _) = Self::text_to_line_index(self.context_pos, &self.rows).unwrap();
        let view_range = Self::compute_vertical_range(context_line, view_height, self.rows.len());
        view_range
    }

    pub fn to_flat_range(&self, line_range: Range<usize>) -> Range<usize> {
        // compute the flat buffer positions of the line range
        let start = Self::line_to_text_index(line_range.start, &self.rows).unwrap();
        let end = Self::line_to_text_index(line_range.end, &self.rows).unwrap();
        start..end
    }

    fn compute_vertical_range(
        context_line: usize,
        view_height: usize,
        total_lines: usize,
    ) -> Range<usize> {
        use std::cmp::min;

        if context_line < view_height / 2 {
            0..min(view_height, total_lines)
        } else if context_line + view_height / 2 >= total_lines {
            total_lines.saturating_sub(view_height)..total_lines
        } else {
            (context_line - view_height / 2)..(context_line + 1 + view_height / 2)
        }
    }

    /// Converts a 1D text buffer position into a tuple containing
    /// line number and a character index into that line
    pub fn text_to_line_index<T: AsRef<str>>(
        index: usize,
        text_lines: &[T],
    ) -> Result<(usize, usize), &'static str> {
        let mut offset = index;
        for (ln_index, line) in text_lines.iter().enumerate() {
            let ln_width = utils::tui::input_width(line.as_ref());
            if (0..ln_width).contains(&offset) {
                return Ok((ln_index, offset));
            }
            offset -= ln_width;
        }
        Err("index out of bounds")
    }

    pub fn line_to_text_index(ln_index: usize, text_lines: &[&str]) -> Result<usize, &'static str> {
        if ln_index > text_lines.len() {
            Err("index out of bounds")
        } else {
            Ok(text_lines
                .into_iter()
                .enumerate()
                .take_while(|(i, _)| i != &ln_index)
                .fold(0, |acc, (_, el)| acc + utils::tui::input_width(&el)))
        }
    }
}

pub struct TypeView<'a> {
    /// The full text buffer
    text: &'a str,

    /// Grapheme index around which the view should be vertically centered
    context_pos: usize,

    /// Generic syntax highlighter
    syntax_styling: Box<dyn Highlighter>,

    /// Sparse styling applied after the syntax highlight pass,
    /// used for cursors and special application logic highlighting
    sparse_styling: HashMap<usize, tui::style::Style>,

    /// Enclosing block component
    block: Block<'a>,

    /// Option to override the background color after all styles are applied
    bg_color: tui::style::Color,
}

impl<'a> TypeView<'a> {
    const TAB_SYMBOL: &'static str = "\u{21e5}";
    const NL_SYMBOL: &'static str = "\u{23ce}";
    /// Instantiate a Typeview widget from a text buffer and use the builder
    /// pattern to set custom rendering options
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            context_pos: 0,
            syntax_styling: Box::new(SyntectHighlight),
            sparse_styling: HashMap::new(),
            block: Default::default(),
            bg_color: tui::style::Color::Reset,
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

    fn process_view(&mut self, area: Rect) -> Vec<Vec<StyledGrapheme<'_>>> {
        // split text buffer by newline
        let lines: Vec<&str> = self.text.split_inclusive('\n').collect();

        // extract the minimum required lines from around the context position (view slice)
        let focus_view = FocusView::new().rows(&lines).context_pos(self.context_pos);
        let view_range = focus_view.get_view_line_range(area.height as usize);

        // compute the flat buffer positions of the view line range
        let Range { start, end } = focus_view.to_flat_range(view_range.clone());
        // apply highlighting
        let view_slice = &lines[view_range.clone()];
        let view_slice = self.syntax_styling.highlight(view_slice);
        // apply text transforms and sparse styling
        let view_slice = self.apply_transforms(start, view_slice);

        Self::into_wrapped_view(view_slice, self.context_pos - start, &area)
    }

    fn apply_transforms<'txt>(
        &mut self,
        mut key_offset: usize,
        lines: Vec<Vec<(&'txt str, tui::style::Color)>>,
    ) -> Vec<StyledGrapheme<'txt>> {
        lines
            .into_iter()
            .map(|tkns_line| Self::tokens_to_graphemes(tkns_line.as_slice()))
            .flat_map(|graphemes_line| {
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
            style: tui::style::Style::default().fg(tui::style::Color::Rgb(100, 100, 100)),
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

    fn into_wrapped_view<'txt>(
        graphemes: Vec<StyledGrapheme<'txt>>,
        context_pos: usize,
        area: &Rect,
    ) -> Vec<Vec<StyledGrapheme<'txt>>> {
        // once transforms are done we can wrap the lines to the output area width
        let mut wrapped_lines = Self::wrap_lines(area.width as u16, graphemes);
        // wrapped_lines

        let wrapped_view = wrapped_lines
            .iter()
            .map(|ln| {
                ln.into_iter()
                    .flat_map(|gphm| gphm.symbol.chars())
                    .collect()
            })
            .collect::<Vec<String>>();

        // refocus the context position after wrapping
        let refocused_view = FocusView::new()
            .rows(&wrapped_view)
            .context_pos(context_pos);
        wrapped_lines
            .drain(refocused_view.get_view_line_range(area.height as usize))
            .collect()
    }

    fn wrap_lines(width: u16, graphemes: Vec<StyledGrapheme>) -> Vec<Vec<StyledGrapheme>> {
        let mut graphemes_it = graphemes.into_iter();
        let mut line_composer = WordWrapper::new(&mut graphemes_it, width, false);
        let mut lines: Vec<Vec<StyledGrapheme>> = vec![];

        while let Some((current_line, _)) = line_composer.next_line() {
            lines.push(current_line.into_iter().cloned().collect());
        }

        lines
    }
}

impl<'a> Widget for TypeView<'a> {
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