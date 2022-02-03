use std::collections::HashMap;
use tui::text::StyledGrapheme;

/// Convert a line to rows given a buffer width
pub trait LineProcessor {
    fn process_line<'txt>(
        &self,
        line: &mut dyn Iterator<Item = StyledGrapheme<'txt>>,
        width: u16,
    ) -> Vec<Vec<StyledGrapheme<'txt>>>;
}
