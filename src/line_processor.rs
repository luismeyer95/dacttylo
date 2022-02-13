use std::collections::HashMap;
use tui::{style::Color, text::StyledGrapheme};

use syntect::highlighting::Theme;

/// Convert text lines to styled rows given a buffer width
pub trait LineProcessor {
    fn process_line<'txt>(
        &self,
        line: &mut dyn Iterator<Item = StyledGrapheme<'txt>>,
        width: u16,
        default_bg: Color,
    ) -> Vec<Vec<StyledGrapheme<'txt>>>;
}
