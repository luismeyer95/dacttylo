use super::highlighter::Highlighter;

/// A no-op default implementation
pub struct NoOpHighlighter;
impl Highlighter for NoOpHighlighter {
    fn highlight<'txt>(
        &mut self,
        lines: &[&'txt str],
    ) -> Vec<Vec<(&'txt str, tui::style::Style)>> {
        lines.iter().map(|&s| self.highlight_line(s)).collect()
    }

    fn highlight_line<'txt>(
        &mut self,
        line: &'txt str,
    ) -> Vec<(&'txt str, tui::style::Style)> {
        vec![(line, tui::style::Style::default())]
    }
}
