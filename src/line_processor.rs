use std::collections::HashMap;
use tui::text::StyledGrapheme;

#[derive(Debug, Clone, PartialEq)]
pub struct MappedCell<'a> {
    pub grapheme: StyledGrapheme<'a>,
    pub index: Option<usize>,
}

impl<'a> MappedCell<'a> {
    pub fn new(index: Option<usize>, grapheme: StyledGrapheme<'a>) -> Self {
        Self { grapheme, index }
    }
}

/// Convert a line to rows given a buffer width
pub trait LineProcessor {
    fn process_line<'txt>(
        &self,
        line: &mut dyn Iterator<Item = StyledGrapheme<'txt>>,
        width: u16,
    ) -> Vec<Vec<MappedCell<'txt>>>;
}
