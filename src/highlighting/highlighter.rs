use tui::text::StyledGrapheme;

/// Highlighter trait for applying global text styling before rendering a Typeview widget
pub trait Highlighter {
    fn highlight<'txt>(
        &self,
        lines: &[&'txt str],
    ) -> Vec<Vec<StyledGrapheme<'txt>>>;
    fn highlight_line<'txt>(
        &self,
        line: &'txt str,
    ) -> Vec<StyledGrapheme<'txt>>;
}
