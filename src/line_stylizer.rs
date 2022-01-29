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

pub struct LineStylizer;

impl LineProcessor for LineStylizer {
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

impl LineStylizer {
    const TAB_SYMBOL: &'static str = "\u{21e5}";
    const NL_SYMBOL: &'static str = "\u{23ce}";

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
            let remapped_key = Self::remap_symbol(column_offset, gphm);
            let styled_key = Self::apply_sparse_styling(
                key_offset,
                remapped_key,
                &sparse_styling,
            );
            let column_size: usize =
                styled_key.iter().map(|k| k.symbol.width()).sum();
            // let column_size = styled_key.len();

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
            key_as_graphemes[0].style = *style;
        }
        key_as_graphemes
    }

    fn wrap_line(
        graphemes: Vec<StyledGrapheme>,
        width: u16,
    ) -> Vec<Vec<StyledGrapheme>> {
        let mut rows: Vec<Vec<StyledGrapheme>> = Vec::with_capacity(16);
        let mut cur_row: Vec<StyledGrapheme> = Vec::with_capacity(16);
        let mut cur_row_width = 0usize;

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

    // fn wrap_line(
    //     graphemes: Vec<StyledGrapheme>,
    //     width: u16,
    // ) -> Vec<Vec<StyledGrapheme>> {
    //     graphemes
    //         .chunks((width) as usize)
    //         .map(|x| x.to_vec())
    //         .collect()
    // }

    // fn wrap_line(
    //     graphemes: Vec<StyledGrapheme>,
    //     width: u16,
    // ) -> Vec<Vec<StyledGrapheme>> {
    //     // use tui::widgets::Paragraph;
    //     let mut gphm_iter = graphemes.into_iter();
    //     let mut ln_wrapper: Box<dyn LineComposer> =
    //         Box::new(WordWrapper::new(&mut gphm_iter, width, false));

    //     let mut rows: Vec<Vec<StyledGrapheme>> = Vec::with_capacity(16);

    //     while let Some((current_line, _)) = ln_wrapper.next_line() {
    //         rows.push(current_line.into());
    //     }

    //     rows
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
        column_index: usize,
    ) -> Vec<StyledGrapheme> {
        let tab_width = (4 - column_index % 4) as u8;
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

        // vec![StyledGrapheme {
        //     symbol: Self::TAB_SYMBOL,
        //     style,
        // }]
        // .into_iter()
        // .chain(vec![
        //     StyledGrapheme { symbol: " ", style };
        //     (tab_width - 1) as usize
        // ])
        // .collect()
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
