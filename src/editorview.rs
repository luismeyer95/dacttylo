use crate::textview::TextView;

struct RenderMetadata {
    line_rows_map: Vec<usize>,
    buffer_height: u16,
}

struct EditorView<'a> {
    text_lines: Vec<&'a str>,
    view_anchor: usize,
    tracked_line: usize,
}

impl<'a> EditorView<'a> {
    pub fn new(text_lines: Vec<&'a str>) -> Self {
        Self {
            text_lines,
            view_anchor: 0,
            tracked_line: 0,
        }
    }

    // pub fn render() {

    // }
}
