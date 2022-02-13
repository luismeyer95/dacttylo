use tui::{
    style::{Color, Style},
    text::StyledGrapheme,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Convert text lines to styled rows given a buffer width
pub trait LineProcessor {
    fn process_line<'txt>(
        &self,
        line: &mut dyn Iterator<Item = StyledGrapheme<'txt>>,
        width: u16,
        default_bg: Color,
    ) -> Vec<Vec<StyledGrapheme<'txt>>>;
}

const SPACE: &str = " ";

pub struct SymbolMap {
    pub tab: StyledGrapheme<'static>,
    pub nl: StyledGrapheme<'static>,
}

pub struct BaseLineProcessor {
    pub symbols: SymbolMap,
}

impl Default for BaseLineProcessor {
    fn default() -> Self {
        let empty_cell = StyledGrapheme {
            symbol: SPACE,
            style: Style::default(),
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
        width: u16,
        default_bg: Color,
    ) -> Vec<Vec<StyledGrapheme<'txt>>> {
        let line = self.transform_line(line, default_bg);
        Self::wrap_line(line, width)
    }
}

impl BaseLineProcessor {
    fn transform_line<'txt>(
        &self,
        line: &mut dyn Iterator<Item = StyledGrapheme<'txt>>,
        default_bg: Color,
    ) -> Vec<StyledGrapheme<'txt>> {
        let mut inline_offset = 0;
        let mut transformed_line: Vec<StyledGrapheme> = vec![];

        for (key_offset, gphm) in line.into_iter().enumerate() {
            let remapped_key = match gphm.symbol {
                "\n" => self.remap_newline(gphm),
                "\t" => self.remap_tab(gphm, inline_offset),
                _ => vec![gphm],
            };
            let column_size: usize =
                remapped_key.iter().map(|k| k.symbol.width()).sum();
            transformed_line.extend(remapped_key);
            inline_offset += column_size;
        }

        transformed_line
    }

    fn remap_tab<'txt>(
        &self,
        grapheme: StyledGrapheme<'txt>,
        inline_index: usize,
    ) -> Vec<StyledGrapheme<'txt>> {
        let tab_width = (4 - inline_index % 4) as u8;
        let style = grapheme.style.patch(Style::default().fg(Color::Yellow));

        let mut tab = vec![StyledGrapheme {
            symbol: self.symbols.tab.symbol,
            style,
        }];

        tab.extend(vec![
            StyledGrapheme { symbol: " ", style };
            (tab_width - 1) as usize
        ]);

        tab
    }

    fn remap_newline<'txt>(
        &self,
        grapheme: StyledGrapheme<'txt>,
    ) -> Vec<StyledGrapheme<'txt>> {
        vec![StyledGrapheme {
            symbol: self.symbols.nl.symbol,
            style: grapheme.style.patch(Style::default().fg(Color::Yellow)),
        }]
    }

    fn wrap_line(
        graphemes: Vec<StyledGrapheme>,
        width: u16,
    ) -> Vec<Vec<StyledGrapheme>> {
        let mut rows: Vec<Vec<StyledGrapheme>> = vec![];
        let mut cur_row: Vec<StyledGrapheme> = vec![];
        let mut cur_row_width = 0;

        let words: Vec<String> = {
            let s = graphemes.iter().map(|g| g.symbol).collect::<String>();
            s.split_word_bounds().map(|x| x.to_string()).collect()
        };

        let mut gphm_iter = graphemes.into_iter();

        for word in words {
            let word_width = word.width();
            if word_width == 0 {
                continue;
            }
            if word_width + cur_row_width > width as usize {
                rows.push(cur_row);
                cur_row = vec![];
                cur_row_width = 0;
            }
            let styled_word: Vec<_> = (&mut gphm_iter)
                .take(word.graphemes(true).count())
                .collect();
            cur_row.extend(styled_word);
            cur_row_width += word_width;
        }

        if !cur_row.is_empty() {
            rows.push(cur_row);
        }

        rows
    }
}
