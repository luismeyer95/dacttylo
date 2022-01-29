use std::collections::HashMap;
use tui::text::StyledGrapheme;

/// Convert text lines to styled rows given a buffer width
pub trait LineProcessor {
    fn process_line<'txt>(
        &self,
        line: &mut dyn Iterator<Item = StyledGrapheme<'txt>>,
        // Sparse styling applied after the syntax highlight pass,
        // used for cursors and special application logic highlighting
        sparse_styling: HashMap<usize, tui::style::Style>,
        width: u16,
    ) -> Vec<Vec<StyledGrapheme<'txt>>>;
}
