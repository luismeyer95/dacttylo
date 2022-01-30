use crate::{
    line_processor::{LineProcessor, MappedCell},
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
        // sparse_styling: HashMap<usize, tui::style::Style>,
        width: u16,
    ) -> Vec<Vec<MappedCell<'txt>>> {
        let line = self.transform_line(line);
        Self::wrap_line(line, width)
    }
}

impl BaseLineProcessor {
    fn transform_line<'txt>(
        &self,
        line: &mut dyn Iterator<Item = StyledGrapheme<'txt>>,
    ) -> Vec<MappedCell<'txt>> {
        let mut graphemes: Vec<StyledGrapheme> = line.collect();
        // appending a blank cell for the end of line style case
        // unconditional to prevent sudden rewrapping on cursor movement
        graphemes.push(StyledGrapheme {
            symbol: " ",
            style: Default::default(),
        });

        let mut column_offset = 0;
        let mut transformed_line: Vec<MappedCell> = vec![];

        for (key_offset, gphm) in graphemes.into_iter().enumerate() {
            let remapped_key =
                self.remap_symbol(column_offset, gphm, key_offset);
            let column_size: usize =
                remapped_key.iter().map(|k| k.grapheme.symbol.width()).sum();
            transformed_line.extend(remapped_key);
            column_offset += column_size;
        }

        transformed_line
        // Self::prefix_line(transformed_line)
    }

    // fn prefix_line(ln: Vec<StyledGrapheme>) -> Vec<StyledGrapheme> {
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

    // ln
    // }

    // fn apply_sparse_styling<'txt>(
    //     key_offset: usize,
    //     mut key_as_graphemes: Vec<StyledGrapheme<'txt>>,
    //     sparse_styling: &HashMap<usize, tui::style::Style>,
    // ) -> Vec<StyledGrapheme<'txt>> {
    //     if let Some(style) = sparse_styling.get(&key_offset) {
    //         // key_as_graphemes[0].style = *style;
    //         let mut style_ref = &mut key_as_graphemes[0].style;
    //         *style_ref = style_ref.patch(*style);
    //     }
    //     key_as_graphemes
    // }

    fn wrap_line(
        mapped_cells: Vec<MappedCell>,
        width: u16,
    ) -> Vec<Vec<MappedCell>> {
        let mut rows: Vec<Vec<MappedCell>> = Vec::with_capacity(16);
        let mut cur_row: Vec<MappedCell> = Vec::with_capacity(16);
        let mut cur_row_width = 0;

        for cell in mapped_cells {
            let sym_width = cell.grapheme.symbol.width();
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

    fn remap_symbol<'txt>(
        &self,
        inline_index: usize,
        grapheme: StyledGrapheme<'txt>,
        key_offset: usize,
    ) -> Vec<MappedCell<'txt>> {
        match grapheme.symbol {
            "\n" => self.remap_newline(grapheme, key_offset),
            "\t" => self.remap_tab(grapheme, inline_index, key_offset),
            _ => vec![MappedCell::new(Some(key_offset), grapheme)],
        }
    }

    fn remap_tab<'txt>(
        &self,
        grapheme: StyledGrapheme,
        column_index: usize,
        key_offset: usize,
    ) -> Vec<MappedCell<'txt>> {
        let tab_width = (4 - column_index % 4) as u8;

        let mapped_tab = MappedCell::new(
            Some(key_offset),
            StyledGrapheme {
                symbol: self.symbols.tab.symbol,
                style: grapheme.style.patch(self.symbols.tab.style),
            },
        );

        let mut tab = vec![mapped_tab];

        tab.extend(vec![
            MappedCell::new(
                None,
                StyledGrapheme {
                    symbol: " ",
                    style: grapheme.style
                }
            );
            (tab_width - 1) as usize
        ]);

        tab
    }

    fn remap_newline<'txt>(
        &self,
        grapheme: StyledGrapheme,
        key_offset: usize,
    ) -> Vec<MappedCell<'txt>> {
        let mapped_nl = MappedCell::new(
            Some(key_offset),
            StyledGrapheme {
                symbol: self.symbols.nl.symbol,
                style: grapheme.style.patch(self.symbols.nl.style),
            },
        );
        vec![mapped_nl]
    }
}

pub struct LineStylizer;

impl LineProcessor for LineStylizer {
    fn process_line<'txt>(
        &self,
        line: &mut dyn Iterator<Item = StyledGrapheme<'txt>>,
        width: u16,
    ) -> Vec<Vec<MappedCell<'txt>>> {
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

        processor.process_line(line, width)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        line_processor::{LineProcessor, MappedCell},
        line_stylizer::BaseLineProcessor,
        utils::types::Coord,
    };
    use tui::text::StyledGrapheme;
    use unicode_segmentation::UnicodeSegmentation;

    #[test]
    fn identity_mapping() {
        use tui::style::{Color, Style};

        let proc = BaseLineProcessor::default();
        let line = "Hello world\n";

        let mut graphemes_iter =
            line.graphemes(true).map(|gp| StyledGrapheme {
                symbol: gp,
                style: Default::default(),
            });

        let res: Vec<Vec<MappedCell>> =
            proc.process_line(&mut graphemes_iter, 4);

        for (index, cell) in res.into_iter().flatten().enumerate() {
            assert_eq!(Some(index), cell.index);
        }
    }

    #[test]
    fn using_tabs() {
        use tui::style::{Color, Style};

        // 'w' should be mapped to (0, 8) in all cases given width > 8
        let lines = vec![
            "Hell\tworld\n",
            "Hello\tworld\n",
            "Helloo\tworld\n",
            "Hellooo\tworld\n",
        ];

        let test = |slice: &str, width| {
            let proc = BaseLineProcessor::default();
            let default_style = Style::default();

            let mut graphemes_iter =
                slice.graphemes(true).map(|gp| StyledGrapheme {
                    symbol: gp,
                    style: default_style,
                });

            let rows: Vec<Vec<MappedCell>> =
                proc.process_line(&mut graphemes_iter, width);

            let w_index = slice.chars().position(|x| x == 'w').unwrap();

            assert_eq!(rows[0][7].index, Some(w_index));
        };

        for ln in lines {
            test(ln, 10);
        }
    }
}
