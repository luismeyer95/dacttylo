/// Highlighter trait for applying global text styling before rendering a Typeview widget
pub trait Highlighter {
    fn highlight<'txt>(
        &mut self,
        lines: &[&'txt str],
    ) -> Vec<Vec<(&'txt str, tui::style::Style)>>;
    fn highlight_line<'txt>(
        &mut self,
        line: &'txt str,
    ) -> Vec<(&'txt str, tui::style::Style)>;
}
