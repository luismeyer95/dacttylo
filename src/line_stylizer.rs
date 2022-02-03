use crate::{
    line_processor::LineProcessor,
    utils::{
        log,
        reflow::{LineComposer, WordWrapper},
    },
};
use std::collections::HashMap;
use tui::text::StyledGrapheme;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const TAB_SYMBOL: &str = "\u{21e5}";
const NL_SYMBOL: &str = "\u{23ce}";
const SPACE: &str = " ";

pub struct SymbolMap {
    pub tab: StyledGrapheme<'static>,
    pub nl: StyledGrapheme<'static>,
}

pub struct BaseLineProcessor {
    symbols: SymbolMap,
}

impl Default for BaseLineProcessor {
    fn default() -> Self {
        let empty_cell = StyledGrapheme {
            symbol: SPACE,
            style: tui::style::Style::default(),
        };

        Self {
            symbols: SymbolMap {
                tab: empty_cell.clone(),
                nl: empty_cell,
            },
        }
    }
}

impl LineProcessor for BaseLineProcessor {
    fn process_line<'txt>(
        &self,
        line: &mut dyn Iterator<Item = StyledGrapheme<'txt>>,
        sparse_styling: HashMap<usize, tui::style::Style>,
        width: u16,
    ) -> Vec<Vec<StyledGrapheme<'txt>>> {
        let line = self.transform_line(line, sparse_styling);
        Self::wrap_line(line, width)
    }
}

impl BaseLineProcessor {
    fn transform_line<'txt>(
        &self,
        line: &mut dyn Iterator<Item = StyledGrapheme<'txt>>,
        sparse_styling: HashMap<usize, tui::style::Style>,
    ) -> Vec<StyledGrapheme<'txt>> {
        let mut graphemes: Vec<StyledGrapheme> = line.collect();
        // appending a blank cell for the end of line style case
        // unconditional to prevent sudden rewrapping on cursor movement
        graphemes.push(StyledGrapheme {
            symbol: " ",
            style: Default::default(),
        });

        let mut column_offset = 0;
        let mut transformed_line: Vec<StyledGrapheme> = vec![];

        for (key_offset, gphm) in graphemes.into_iter().enumerate() {
            let remapped_key = self.remap_symbol(column_offset, gphm);
            let styled_key = Self::apply_sparse_styling(
                key_offset,
                remapped_key,
                &sparse_styling,
            );
            let column_size: usize =
                styled_key.iter().map(|k| k.symbol.width()).sum();

            transformed_line.extend(styled_key);
            column_offset += column_size;
        }

        Self::prefix_line(transformed_line)
    }

    fn prefix_line(ln: Vec<StyledGrapheme>) -> Vec<StyledGrapheme> {
        // let mut prefixed = vec![
        //     StyledGrapheme {
        //         symbol: "~",
        //         style: Default::default(),
        //     },
        //     StyledGrapheme {
        //         symbol: " ",
        //         style: Default::default(),
        //     },
        // ];
        // prefixed.extend(ln);
        // prefixed

        ln
    }

    fn apply_sparse_styling<'txt>(
        key_offset: usize,
        mut key_as_graphemes: Vec<StyledGrapheme<'txt>>,
        sparse_styling: &HashMap<usize, tui::style::Style>,
    ) -> Vec<StyledGrapheme<'txt>> {
        if let Some(style) = sparse_styling.get(&key_offset) {
            // key_as_graphemes[0].style = *style;
            let mut style_ref = &mut key_as_graphemes[0].style;
            *style_ref = style_ref.patch(*style);
        }
        key_as_graphemes
    }

    fn wrap_line(
        graphemes: Vec<StyledGrapheme>,
        width: u16,
    ) -> Vec<Vec<StyledGrapheme>> {
        let mut rows: Vec<Vec<StyledGrapheme>> = Vec::with_capacity(16);
        let mut cur_row: Vec<StyledGrapheme> = Vec::with_capacity(16);
        let mut cur_row_width = 0;

        for g in graphemes {
            let sym_width = g.symbol.width();
            if sym_width + cur_row_width > width as usize {
                rows.push(cur_row);
                cur_row = vec![];
                cur_row_width = 0;
            }
            cur_row.push(g);
            cur_row_width += sym_width;
        }

        if !cur_row.is_empty() {
            rows.push(cur_row);
        }

        rows
    }

    fn remap_symbol<'txt>(
        &self,
        inline_index: usize,
        grapheme: StyledGrapheme<'txt>,
    ) -> Vec<StyledGrapheme<'txt>> {
        match grapheme.symbol {
            "\n" => self.remap_newline(grapheme),
            "\t" => self.remap_tab(grapheme, inline_index),
            _ => vec![grapheme],
        }
    }

    fn remap_tab<'txt>(
        &self,
        grapheme: StyledGrapheme,
        column_index: usize,
    ) -> Vec<StyledGrapheme<'txt>> {
        let tab_width = (4 - column_index % 4) as u8;

        let mut tab = vec![StyledGrapheme {
            symbol: self.symbols.tab.symbol,
            style: grapheme.style.patch(self.symbols.tab.style),
        }];
        tab.extend(vec![
            StyledGrapheme {
                symbol: " ",
                style: grapheme.style
            };
            (tab_width - 1) as usize
        ]);

        tab
    }

    fn remap_newline<'txt>(
        &self,
        grapheme: StyledGrapheme,
    ) -> Vec<StyledGrapheme<'txt>> {
        vec![StyledGrapheme {
            symbol: self.symbols.nl.symbol,
            style: grapheme.style.patch(self.symbols.nl.style),
        }]
    }
}

pub struct LineStylizer;

impl LineProcessor for LineStylizer {
    fn process_line<'txt>(
        &self,
        line: &mut dyn Iterator<Item = StyledGrapheme<'txt>>,
        sparse_styling: HashMap<usize, tui::style::Style>,
        width: u16,
    ) -> Vec<Vec<StyledGrapheme<'txt>>> {
        let yellow = |symbol| StyledGrapheme {
            style: tui::style::Style::default().fg(tui::style::Color::Yellow),
            symbol,
        };

        let processor = BaseLineProcessor {
            symbols: SymbolMap {
                tab: yellow(TAB_SYMBOL),
                nl: yellow(NL_SYMBOL),
            },
        };

        processor.process_line(line, sparse_styling, width)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
}
