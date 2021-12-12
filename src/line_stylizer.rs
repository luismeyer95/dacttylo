use crate::line_processor::LineProcessor;
use std::collections::HashMap;
use tui::text::StyledGrapheme;
use unicode_segmentation::UnicodeSegmentation;

pub struct LineStylizer;

impl LineProcessor for LineStylizer {
    fn process_line<'txt>(
        &mut self,
        line: &[(&'txt str, tui::style::Style)],
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

    fn transform_line<'txt>(
        &mut self,
        line: &[(&'txt str, tui::style::Style)],
        sparse_styling: HashMap<usize, tui::style::Style>,
    ) -> Vec<StyledGrapheme<'txt>> {
        let mut graphemes = Self::tokens_to_graphemes(line);
        // appending a blank cell for the end of line style case
        // unconditional to prevent sudden rewrapping on cursor movement
        graphemes.push(StyledGrapheme {
            symbol: " ",
            style: Default::default(),
        });

        let mut inline_offset = 0;
        let mut key_offset = 0;
        let mut transformed_line: Vec<StyledGrapheme> = vec![];

        for gphm in graphemes.into_iter() {
            let remapped_key = Self::remap_symbol(inline_offset, gphm.clone());
            let styled_key = Self::apply_sparse_styling(key_offset, remapped_key, &sparse_styling);
            let size = styled_key.iter().count();
            transformed_line.extend(styled_key);
            key_offset += 1;
            inline_offset += size;
        }

        Self::prefix_line(transformed_line)
    }

    fn prefix_line(ln: Vec<StyledGrapheme>) -> Vec<StyledGrapheme> {
        let mut prefixed = vec![
            StyledGrapheme {
                symbol: "~",
                style: Default::default(),
            },
            StyledGrapheme {
                symbol: " ",
                style: Default::default(),
            },
        ];
        prefixed.extend(ln);
        prefixed
    }

    fn tokens_to_graphemes<'tkn>(
        tokens: &[(&'tkn str, tui::style::Style)],
    ) -> Vec<StyledGrapheme<'tkn>> {
        tokens
            .iter()
            .flat_map(|(token, style)| {
                token.graphemes(true).map(|g| StyledGrapheme {
                    symbol: g,
                    style: *style,
                })
            })
            .collect::<Vec<StyledGrapheme<'tkn>>>()
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

    fn wrap_line(graphemes: Vec<StyledGrapheme>, width: u16) -> Vec<Vec<StyledGrapheme>> {
        graphemes
            .chunks((width) as usize)
            .map(|x| x.to_vec())
            .collect()
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
        vec![StyledGrapheme {
            symbol: Self::NL_SYMBOL,
            style: grapheme
                .style
                .patch(tui::style::Style::default().fg(tui::style::Color::Yellow)),
        }]
    }
}
