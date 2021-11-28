use tui::text::StyledGrapheme;
use unicode_segmentation::UnicodeSegmentation;

pub fn input_width(s: &str) -> usize {
    UnicodeSegmentation::graphemes(s, true).count()
}
