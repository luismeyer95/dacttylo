use unicode_width::UnicodeWidthStr;

use crate::{text_coord::TextCoord, utils::helpers::StrGraphemesExt};
use std::cmp::min;

pub enum Cursor {
    Up,
    Down,
    Left,
    Right,
}

pub struct EditorState {
    text_lines: Vec<String>,
    cursor: TextCoord,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            text_lines: vec!["".into()],
            cursor: TextCoord::new(0, 0),
        }
    }

    pub fn content(mut self, text: &str) -> Self {
        let mut lines = text
            // TODO: handle \r\n
            .split_inclusive("\n")
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        if lines.is_empty() {
            lines.push("".into());
        }
        self.text_lines = lines;
        self
    }

    pub fn move_cursor(&mut self, cmd: Cursor) {
        match cmd {
            Cursor::Up => {
                self.cursor.ln = self.cursor.ln.saturating_sub(1);
                self.cursor.x = min(
                    self.cursor.x,
                    Self::nl_stripped_len(&self.text_lines[self.cursor.ln]),
                );
            }
            Cursor::Down => {
                self.cursor.ln = min(
                    self.cursor.ln.saturating_add(1),
                    self.text_lines.len() - 1,
                );
                self.cursor.x = min(
                    self.cursor.x,
                    Self::nl_stripped_len(&self.text_lines[self.cursor.ln]),
                );
            }
            Cursor::Left => self.cursor.x = self.cursor.x.saturating_sub(1),
            Cursor::Right => {
                self.cursor.x = min(
                    self.cursor.x.saturating_add(1),
                    Self::nl_stripped_len(&self.text_lines[self.cursor.ln]),
                );
            }
        }
    }

    fn nl_stripped_len(s: &str) -> usize {
        if s.ends_with('\n') {
            s.len_graphemes() - 1
        } else {
            s.len_graphemes()
        }
    }

    pub fn offset(&mut self, offset: isize) -> Option<TextCoord> {
        let left_offset = offset.unsigned_abs();
        let new_coord = if offset < 0 {
            self.offset_neg(left_offset, self.cursor.clone())?
        } else {
            self.offset_pos(left_offset, self.cursor.clone())?
        };
        self.cursor = new_coord;
        Some(self.cursor.clone())
    }

    fn offset_pos(
        &self,
        mut offset: usize,
        mut coord: TextCoord,
    ) -> Option<TextCoord> {
        let mut ln = self.text_lines.get(coord.ln)?;
        let mut ln_len = ln.len_graphemes();

        let mut next_ln = self.text_lines.get(coord.ln + 1);
        while offset >= ln_len - coord.x {
            // handling the special case for the last line end-of-line cursor
            if next_ln == None && offset == ln_len - coord.x {
                break;
            }
            offset -= ln_len - coord.x;
            coord.ln += 1;
            coord.x = 0;
            ln = next_ln?;
            ln_len = ln.len_graphemes();
            next_ln = self.text_lines.get(coord.ln + 1);
        }
        coord.x += offset;
        Some(coord)
    }

    fn offset_neg(
        &self,
        mut offset: usize,
        mut coord: TextCoord,
    ) -> Option<TextCoord> {
        let mut ln;
        while offset > coord.x {
            offset -= coord.x + 1;
            ln = self.text_lines.get(coord.ln.checked_sub(1)?)?;
            coord.ln -= 1;
            // if a previous line exists, it is not the last line therefore it must
            // have a trailing newline and a minimum length of 1
            coord.x = ln.len_graphemes() - 1;
        }
        coord.x -= offset;
        Some(coord)
    }

    pub fn insert_ch(&mut self, c: char) {
        let ln = &mut self.text_lines[self.cursor.ln];
        match c {
            '\n' => {
                let x = ln.index_graphemes(self.cursor.x);
                let carry = ln[x..].to_string();
                ln.replace_range(x.., "\n");
                self.text_lines.insert(self.cursor.ln + 1, carry);
            }
            _ => {
                let insert_point = ln.index_graphemes(self.cursor.x);
                ln.insert(insert_point, c);

                // ln.insert(self.cursor.x, c);
            }
        }
    }

    pub fn delete_ch(&mut self) {
        let cursor_ch = self.cursor_ch();
        match cursor_ch {
            Some('\n') => {
                // assumes presence of newline guarantees this line isn't the last
                let ln_below = self.text_lines.remove(self.cursor.ln + 1);
                let ln = &mut self.text_lines[self.cursor.ln];
                let rm_point = ln.index_graphemes(self.cursor.x);
                ln.replace_range(rm_point.., &ln_below);
            }
            Some(_) => {
                let ln = &mut self.text_lines[self.cursor.ln];
                let rm_point = ln.index_graphemes(self.cursor.x);
                ln.remove(rm_point);
            }
            None => {}
        };
    }

    pub fn cursor_ch(&self) -> Option<char> {
        let ln = self.text_lines.get(self.cursor.ln)?;
        ln.chars().nth(self.cursor.x)
    }

    pub fn get_cursor(&self) -> TextCoord {
        TextCoord::new(self.cursor.ln, self.cursor.x)
    }

    pub fn get_lines(&self) -> Vec<&str> {
        self.text_lines.iter().map(|s| s.as_str()).collect()
    }
}
