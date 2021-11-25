use tui::text::StyledGrapheme;
use unicode_segmentation::UnicodeSegmentation;

pub fn input_width(s: &str) -> usize {
    UnicodeSegmentation::graphemes(s, true).count()
}

/// Converts a 1D text buffer position into a tuple containing
/// line number and a character index into that line
pub fn text_to_line_index<T: AsRef<str>>(
    index: usize,
    text_lines: &[T],
) -> Result<(usize, usize), &'static str> {
    let mut offset = index;
    for (ln_index, line) in text_lines.iter().enumerate() {
        let ln_width = input_width(line.as_ref());
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
            .take_while(|(i, el)| i != &ln_index)
            .fold(0, |acc, (i, el)| acc + input_width(&el)))
    }
}

pub fn styled_graphemes<'tkn>(
    text: &[(&'tkn str, tui::style::Style)],
) -> Vec<StyledGrapheme<'tkn>> {
    text.into_iter()
        .flat_map(|(token, style)| {
            token.graphemes(true).map(|g| StyledGrapheme {
                symbol: g,
                style: style.clone(),
            })
        })
        .collect()
}
