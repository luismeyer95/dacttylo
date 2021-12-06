use std::collections::HashMap;

use crate::{
    highlight::{Highlighter, NoHighlight},
    line_processor::LineProcessor,
    utils::reflow::{LineComposer, WordWrapper},
};
use tui::text::StyledGrapheme;
use unicode_segmentation::UnicodeSegmentation;

pub struct LineStylizer {
    /// Generic syntax highlighter
    syntax_styling: Box<dyn Highlighter>,
}

impl LineProcessor for LineStylizer {
    fn process_line<'txt>(
        &mut self,
        line: &'txt str,
        sparse_styling: HashMap<usize, tui::style::Style>,
        width: u16,
    ) -> Vec<Vec<StyledGrapheme<'txt>>> {
        let line = self.transform_line(line, sparse_styling);
        Self::wrap_line(line, width)
    }
}

impl LineStylizer {
    const TAB_SYMBOL: &'static str = "\u{21e5}";
    const NL_SYMBOL: &'static str = "\u{23ce}";

    pub fn new() -> Self {
        LineStylizer {
            syntax_styling: Box::new(NoHighlight),
        }
    }

    pub fn syntax_styling(mut self, syntax_styling: Box<dyn Highlighter>) -> Self {
        self.syntax_styling = syntax_styling;
        self
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

    fn wrap_line(graphemes: Vec<StyledGrapheme>, width: u16) -> Vec<Vec<StyledGrapheme>> {
        let mut graphemes_it = graphemes.into_iter();
        let mut line_composer = WordWrapper::new(&mut graphemes_it, width, false);
        let mut lines: Vec<Vec<StyledGrapheme>> = vec![];

        while let Some((current_line, _)) = line_composer.next_line() {
            lines.push(current_line.into_iter().cloned().collect());
        }

        lines
    }

    fn transform_line<'txt>(
        &mut self,
        line: &'txt str,
        sparse_styling: HashMap<usize, tui::style::Style>,
    ) -> Vec<StyledGrapheme<'txt>> {
        let highlighted = self.syntax_styling.highlight_line(line);
        let graphemes = Self::tokens_to_graphemes(&highlighted);

        let mut inline_offset = 0;
        let mut key_offset = 0;
        let mut transformed_line: Vec<StyledGrapheme> = vec![];
        transformed_line.push(StyledGrapheme {
            symbol: "~ ",
            style: tui::style::Style::default(),
        });

        for gphm in graphemes.into_iter() {
            let remapped_key = Self::remap_symbol(inline_offset, gphm.clone());
            let styled_key = Self::apply_sparse_styling(key_offset, remapped_key, &sparse_styling);
            let size = styled_key.iter().count();

            transformed_line.extend(styled_key);

            key_offset += 1;
            inline_offset += size;
        }
        transformed_line
    }

    fn apply_sparse_styling<'txt>(
        key_offset: usize,
        mut key_as_graphemes: Vec<StyledGrapheme<'txt>>,
        sparse_styling: &HashMap<usize, tui::style::Style>,
    ) -> Vec<StyledGrapheme<'txt>> {
        sparse_styling
            .get(&key_offset)
            .map(|style| key_as_graphemes[0].style = *style);
        key_as_graphemes
    }

    fn remap_symbol<'txt>(
        inline_index: usize,
        grapheme: StyledGrapheme<'txt>,
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
}
