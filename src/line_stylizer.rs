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
const EMPTY: &str = "";

pub struct SymbolMap {
    pub tab: StyledGrapheme<'static>,
    pub nl: StyledGrapheme<'static>,
}

pub struct BaseLineProcessor {
    symbols: SymbolMap,
}

impl Default for BaseLineProcessor {
    fn default() -> Self {
        let plain = |symbol| StyledGrapheme {
            symbol,
            style: tui::style::Style::default(),
        };

        Self {
            symbols: SymbolMap {
                tab: plain(SPACE),
                nl: plain(EMPTY),
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
        let mut column_offset = 0;
        let mut transformed_line: Vec<MappedCell> = vec![];

        for (key_offset, gphm) in line.enumerate() {
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

    fn wrap_line(
        mapped_cells: Vec<MappedCell>,
        width: u16,
    ) -> Vec<Vec<MappedCell>> {
        let mut rows: Vec<Vec<MappedCell>> = Vec::with_capacity(16);
        let mut cur_row: Vec<MappedCell> = Vec::with_capacity(16);
        let mut cur_row_width = 0;

        for cell in mapped_cells {
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
    use tui::style::{Color, Style};
    use tui::text::StyledGrapheme;
    use unicode_segmentation::UnicodeSegmentation;
    use unicode_width::UnicodeWidthChar;

    fn process_slice(slice: &str, width: u16) -> Vec<Vec<MappedCell>> {
        let proc = BaseLineProcessor::default();
        let mut graphemes_iter =
            slice.graphemes(true).map(|gp| StyledGrapheme {
                symbol: gp,
                style: Default::default(),
            });

        let rows: Vec<Vec<MappedCell>> =
            proc.process_line(&mut graphemes_iter, width);

        rows
    }

    fn expect_rows(expected: &[&str], rows: &[Vec<MappedCell>]) {
        for (row_index, row) in rows.iter().enumerate() {
            let expected_ln: Vec<&str> =
                expected[row_index].graphemes(true).collect();
            for (cell_index, cell) in row.iter().enumerate() {
                let mapped_cell = &rows[row_index][cell_index];
                let expected_grapheme = expected_ln[cell_index];

                assert_eq!(expected_grapheme, mapped_cell.grapheme.symbol);
                if mapped_cell.index.is_none() {
                    assert_eq!(mapped_cell.grapheme.symbol, " ");
                }
            }
        }
    }

    #[test]
    fn identity_mapping() {
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
        // 'w' should be mapped to (0, 8) in all cases given width > 8
        let lines = vec![
            "Hell\tworld\n",
            "Hello\tworld\n",
            "Helloo\tworld\n",
            "Hellooo\tworld\n",
        ];

        for ln in lines {
            let rows = process_slice(ln, 10);
            assert_eq!(
                rows[0][8].index,
                Some(ln.chars().position(|c| c == 'w').unwrap())
            );
        }
    }

    #[test]
    fn using_tabs_with_wrapping() {
        let line = "abc\tdefg\thijk";

        let rows = process_slice(line, 3);

        #[rustfmt::skip]
        let expected = vec![
            "abc",
            " de",
            "fg ",
            "   ",
            "hij",
            "k"
        ];

        expect_rows(&expected, &rows);
    }

    #[test]
    fn unicode() {
        let line = "地こそabcd七e祖腰\n";

        let rows = process_slice(line, 5);
        #[rustfmt::skip]
        let expected = vec![
            "地こ",
            "そabc",
            "d七e",
            "祖腰",
        ];
        expect_rows(&expected, &rows);
        assert_eq!(
            &Some(line.graphemes(true).position(|c| c == "そ").unwrap()),
            &rows[1][0].index,
        );
    }

    #[test]
    fn unicode_with_tabs() {
        let line = "地こそ\tabcd\t七e祖腰\n";

        let rows = process_slice(line, 5);
        #[rustfmt::skip]
        let expected = vec![
            "地こ",
            "そ  a",
            "bcd  ",
            "  七e",
            "祖腰",
        ];
        expect_rows(&expected, &rows);
        assert_eq!(
            &Some(line.graphemes(true).position(|c| c == "e").unwrap()),
            &rows[3][3].index,
        );
    }
}
