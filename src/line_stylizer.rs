use crate::line_processor::LineProcessor;
use std::collections::HashMap;
use tui::{style::Style, text::StyledGrapheme};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

struct StyledWord<'w> {
    symbol: &'w str,
    style: Style,
}

pub struct LineStylizer;

impl LineProcessor for LineStylizer {
    fn process_line<'txt>(
        &self,
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
        &self,
        line: &[(&'txt str, tui::style::Style)],
        sparse_styling: HashMap<usize, tui::style::Style>,
    ) -> Vec<StyledGrapheme<'txt>> {
        let graphemes = Self::tokens_to_graphemes(line);

        let mut inline_offset = 0;
        let mut transformed_line: Vec<StyledGrapheme> = vec![];

        for (key_offset, gphm) in graphemes.into_iter().enumerate() {
            let remapped_key = Self::remap_symbol(inline_offset, gphm.clone());
            let styled_key = Self::apply_sparse_styling(
                key_offset,
                remapped_key,
                &sparse_styling,
            );
            let column_size: usize =
                styled_key.iter().map(|k| k.symbol.width()).sum();
            transformed_line.extend(styled_key);
            inline_offset += column_size;
        }

        transformed_line
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

    fn wrap_line(
        graphemes: Vec<StyledGrapheme>,
        width: u16,
    ) -> Vec<Vec<StyledGrapheme>> {
        let mut rows: Vec<Vec<StyledGrapheme>> = Vec::with_capacity(16);
        let mut cur_row: Vec<StyledGrapheme> = Vec::with_capacity(16);
        let mut cur_row_width = 0;

        for cell in graphemes {
            let sym_width = cell.grapheme.symbol.width();
            if sym_width == 0 {
                continue;
            }
            if sym_width + cur_row_width > width as usize {
                rows.push(cur_row);
                cur_row = vec![];
                cur_row_width = 0;
            }
            cur_row.push(cell);
            cur_row_width += sym_width;
        }

        if !cur_row.is_empty() {
            rows.push(cur_row);
        }

        rows
    }

    fn apply_sparse_styling<'txt>(
        key_offset: usize,
        mut key_as_graphemes: Vec<StyledGrapheme<'txt>>,
        sparse_styling: &HashMap<usize, tui::style::Style>,
    ) -> Vec<StyledGrapheme<'txt>> {
        if let Some(style) = sparse_styling.get(&key_offset) {
            key_as_graphemes[0].style = *style;
        }
        key_as_graphemes
    }

    // fn wrap_line(
    //     graphemes: Vec<StyledGrapheme>,
    //     width: u16,
    // ) -> Vec<Vec<StyledGrapheme>> {
    //     graphemes
    //         .chunks((width) as usize)
    //         .map(|x| x.to_vec())
    //         .collect()
    // }

    fn remap_symbol(
        inline_index: usize,
        grapheme: StyledGrapheme,
    ) -> Vec<StyledGrapheme> {
        match grapheme.symbol {
            "\n" => Self::remap_newline(grapheme),
            "\t" => Self::remap_tab(grapheme, inline_index),
            _ => vec![grapheme],
        }
    }

    fn remap_tab(
        grapheme: StyledGrapheme,
        inline_index: usize,
    ) -> Vec<StyledGrapheme> {
        let tab_width = (4 - inline_index % 4) as u8;
        let style = grapheme
            .style
            .patch(tui::style::Style::default().fg(tui::style::Color::Yellow));

        let mut tab = vec![StyledGrapheme {
            symbol: Self::TAB_SYMBOL,
            style,
        }];

        tab.extend(vec![
            StyledGrapheme { symbol: " ", style };
            (tab_width - 1) as usize
        ]);

        tab
    }

    fn remap_newline(grapheme: StyledGrapheme) -> Vec<StyledGrapheme> {
        vec![StyledGrapheme {
            symbol: Self::NL_SYMBOL,
            style: grapheme.style.patch(
                tui::style::Style::default().fg(tui::style::Color::Yellow),
            ),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping() {}
}
