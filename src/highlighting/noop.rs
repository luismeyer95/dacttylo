use super::highlighter::Highlighter;
use tui::text::StyledGrapheme;
use unicode_segmentation::UnicodeSegmentation;

/// A no-op default implementation
pub struct NoOpHighlighter;
impl Highlighter for NoOpHighlighter {
    fn highlight<'txt>(
        &self,
        lines: &[&'txt str],
    ) -> Vec<Vec<StyledGrapheme<'txt>>> {
        lines.iter().map(|&s| self.highlight_line(s)).collect()
    }

    fn highlight_line<'txt>(
        &self,
        line: &'txt str,
    ) -> Vec<StyledGrapheme<'txt>> {
        line.graphemes(true)
            .map(|g| StyledGrapheme {
                symbol: g,
                style: Default::default(),
            })
            .collect()
    }
}
