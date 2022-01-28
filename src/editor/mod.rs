mod rope_iter;

use std::ops::Range;

pub use rope_iter::RopeGraphemes;
use ropey::Rope;
use unicode_segmentation::UnicodeSegmentation;

use self::rope_iter::{next_grapheme_boundary, prev_grapheme_boundary};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Coord(usize, usize);
pub struct EditorState {
    // Cursor coordinates expressed in (line, char index)
    index: usize,
    text: Rope,
    buffered_column_offset: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            index: 0,
            buffered_column_offset: 0,
            text: Rope::from_str(""),
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.text.insert_char(self.index, c);
    }

    pub fn insert(&mut self, slice: &str) {
        self.text.insert(self.index, slice);
    }

    pub fn append_char(&mut self, c: char) {
        self.insert_char(c);
        self.index += 1;
    }

    pub fn append(&mut self, slice: &str) {
        self.insert(slice);
        self.index += slice.chars().count();
    }

    pub fn at_cursor(&self) -> char {
        self.text.char(self.index)
    }

    pub fn cursor(&self) -> Coord {
        let ln = self.text.char_to_line(self.index);
        let ln_start = self.text.line_to_char(ln);
        let x = self.count_graphemes(ln_start..self.index);
        Coord(ln, x)
    }

    fn count_graphemes(&self, range: Range<usize>) -> usize {
        RopeGraphemes::new(&self.text.slice(range)).count()
    }

    fn column_offset(&self) -> usize {
        let ln = self.text.char_to_line(self.index);
        let ln_start = self.text.line_to_char(ln);
        self.count_graphemes(ln_start..self.index)
    }

    pub fn move_cursor(&mut self, dir: Direction) {
        match dir {
            Direction::Right => {
                self.index = next_grapheme_boundary(
                    &self.text.slice(self.index..),
                    self.index,
                );
            }
            Direction::Left => {
                self.index = prev_grapheme_boundary(
                    &self.text.slice(..self.index),
                    self.index,
                );
            }
            Direction::Up => {
                let ln = self.text.char_to_line(self.index);
                if ln == 0 {
                    return;
                }

                let current_ln_start = self.text.line_to_char(ln);
                let prev_ln_start = self.text.line_to_char(ln - 1);

                self.index = prev_ln_start
                    + std::cmp::min(
                        self.count_graphemes(
                            prev_ln_start..current_ln_start - 1,
                        ),
                        self.count_graphemes(current_ln_start..self.index),
                    )
            }
            Direction::Down => {
                let ln = self.text.char_to_line(self.index);
                if ln == self.text.len_lines() - 1 {
                    return;
                }

                let current_ln_start = self.text.line_to_char(ln);
                let next_ln_start = self.text.line_to_char(ln + 1);
                let next_ln_end = self.text.line_to_char(ln + 2);

                self.index = next_ln_start
                    + std::cmp::min(
                        self.count_graphemes(
                            current_ln_start..next_ln_start - 1,
                        ),
                        self.count_graphemes(next_ln_start..next_ln_end),
                    )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_char() {
        let mut editor = EditorState::new();
        editor.insert_char('a');

        assert_eq!(editor.cursor(), Coord(0, 0));
        assert_eq!(editor.at_cursor(), 'a');

        editor.insert_char('혓');

        assert_eq!(editor.cursor(), Coord(0, 0));
        assert_eq!(editor.at_cursor(), '혓');
    }

    #[test]
    fn append_char() {
        let mut editor = EditorState::new();
        editor.append_char('a');

        assert_eq!(editor.cursor(), Coord(0, 1));
    }

    #[test]
    fn append_char_unicode() {
        let mut editor = EditorState::new();
        editor.append_char('혓');

        assert_eq!(editor.cursor(), Coord(0, 1));
    }

    #[test]
    fn unicode_char_by_char_append() {
        let mut editor = EditorState::new();

        let chars = ['न', 'म', 'स', '्', 'त', 'े'];
        assert_eq!(String::from_iter(chars), "नमस्ते");

        for c in chars {
            editor.append_char(c);
        }

        assert_eq!(editor.cursor(), Coord(0, 4));
    }

    #[test]
    fn nl_append() {
        let mut editor = EditorState::new();

        editor.append_char('\n');
        assert_eq!(editor.cursor(), Coord(1, 0));
    }

    #[test]
    fn move_cursor_empty_buffer() {
        let mut editor = EditorState::new();

        editor.move_cursor(Direction::Right);
        assert_eq!(editor.cursor(), Coord(0, 0));
        editor.move_cursor(Direction::Left);
        assert_eq!(editor.cursor(), Coord(0, 0));
        editor.move_cursor(Direction::Up);
        assert_eq!(editor.cursor(), Coord(0, 0));
        editor.move_cursor(Direction::Down);
        assert_eq!(editor.cursor(), Coord(0, 0));
    }

    #[test]
    fn move_cursor_after_insert() {
        let mut editor = EditorState::new();

        editor.insert_char('a');

        editor.move_cursor(Direction::Right);
        assert_eq!(editor.cursor(), Coord(0, 1));

        editor.move_cursor(Direction::Left);
        assert_eq!(editor.cursor(), Coord(0, 0));
    }

    #[test]
    fn insert_slice() {
        let mut editor = EditorState::new();

        editor.insert("abc");
        assert_eq!(editor.cursor(), Coord(0, 0));
    }

    #[test]
    fn insert_slice_unicode() {
        let mut editor = EditorState::new();

        editor.insert("नमस्ते");
        assert_eq!(editor.cursor(), Coord(0, 0));
    }

    #[test]
    fn append_slice() {
        let mut editor = EditorState::new();

        editor.append("abc");
        assert_eq!(editor.cursor(), Coord(0, 3));
    }

    #[test]
    fn append_slice_unicode() {
        let mut editor = EditorState::new();

        editor.append("नमस्ते");
        assert_eq!(editor.cursor(), Coord(0, 4));
    }

    #[test]
    fn append_slice_with_linebreaks() {
        let mut editor = EditorState::new();

        editor.append("नमस्ते\r\nab\n혓주");
        assert_eq!(editor.cursor(), Coord(2, 2));
    }

    #[test]
    fn move_up() {
        let mut editor = EditorState::new();

        editor.append("abcd\ndefg\n");
        assert_eq!(editor.cursor(), Coord(2, 0));

        editor.move_cursor(Direction::Up);
        assert_eq!(editor.cursor(), Coord(1, 0));

        editor.move_cursor(Direction::Up);
        assert_eq!(editor.cursor(), Coord(0, 0));

        editor.move_cursor(Direction::Up);
        assert_eq!(editor.cursor(), Coord(0, 0));
    }

    #[test]
    fn move_up_adjust_offset() {
        let mut editor = EditorState::new();

        editor.append("\nijkl\nabcdefgh");
        assert_eq!(editor.cursor(), Coord(2, 8));

        editor.move_cursor(Direction::Up);
        assert_eq!(editor.cursor(), Coord(1, 4));

        editor.move_cursor(Direction::Up);
        assert_eq!(editor.cursor(), Coord(0, 0));

        editor.move_cursor(Direction::Up);
        assert_eq!(editor.cursor(), Coord(0, 0));
    }

    #[test]
    fn remember_longest_offset() {
        let mut editor = EditorState::new();

        editor.append("\nijkl");
        assert_eq!(editor.cursor(), Coord(1, 4));

        editor.move_cursor(Direction::Up);
        assert_eq!(editor.cursor(), Coord(0, 0));

        editor.move_cursor(Direction::Down);
        assert_eq!(editor.cursor(), Coord(1, 4));
    }
}
