use tui::text::Text;

use crate::text_coord::TextCoord;
use std::{cmp::min, ops::Not};

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

impl EditorState {
    pub fn new() -> Self {
        Self {
            text_lines: vec!["".into()],
            cursor: TextCoord::new(0, 0),
        }
    }

    pub fn move_cursor(&mut self, cmd: Cursor) {
        let cursor_ch = self.cursor_ch();
        match cmd {
            Cursor::Up => {
                self.cursor.ln = self.cursor.ln.saturating_sub(1);
                self.cursor.x = min(
                    self.cursor.x,
                    Self::nl_stripped_len(&self.text_lines[self.cursor.ln]),
                );
            }
            Cursor::Down => {
                self.cursor.ln = min(self.cursor.ln.saturating_add(1), self.text_lines.len() - 1);
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

    fn char_at(&self, coord: TextCoord) -> Option<char> {
        self.text_lines
            .get(coord.ln)
            .and_then(|ln| ln.chars().nth(coord.x))
    }

    fn nl_stripped_len(s: &str) -> usize {
        if s.ends_with("\n") {
            s.len() - 1
        } else {
            s.len()
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

    fn offset_pos(&self, mut offset: usize, mut coord: TextCoord) -> Option<TextCoord> {
        let mut ln = self.text_lines.get(coord.ln)?;
        let mut next_ln = self.text_lines.get(coord.ln + 1);
        while offset >= ln.len() - coord.x {
            // handling the special case for the last line end-of-line cursor
            if next_ln == None && offset == ln.len() - coord.x {
                break;
            }
            offset -= ln.len() - coord.x;
            coord.ln += 1;
            coord.x = 0;
            ln = next_ln?;
            next_ln = self.text_lines.get(coord.ln + 1);
        }
        coord.x += offset;
        Some(coord)
    }

    fn offset_neg(&self, mut offset: usize, mut coord: TextCoord) -> Option<TextCoord> {
        let mut ln;
        while offset > coord.x {
            offset -= coord.x + 1;
            ln = self.text_lines.get(coord.ln.checked_sub(1)?)?;
            coord.ln -= 1;
            // if a previous line exists, it is not the last line therefore it must
            // have a trailing newline and a minimum length of 1
            coord.x = ln.len() - 1;
        }
        coord.x -= offset;
        Some(coord)
    }

    pub fn insert_ch(&mut self, c: char) {
        let ln = &mut self.text_lines[self.cursor.ln];
        match c {
            '\n' => {
                let x = self.cursor.x;
                let carry = ln[x..].to_string();
                ln.replace_range(x.., "\n");
                self.text_lines.insert(self.cursor.ln + 1, carry);
            }
            _ => {
                ln.insert(self.cursor.x, c);
            }
        }
    }

    pub fn delete_ch(&mut self) {
        let cursor_ch = self.cursor_ch();
        match cursor_ch {
            Some('\n') => {
                // assumes presence of newline guarantees this line isn't the last
                let ln_below = self.text_lines.remove(self.cursor.ln + 1);
                self.text_lines[self.cursor.ln].replace_range(self.cursor.x.., &ln_below);
            }
            Some(_) => {
                self.text_lines[self.cursor.ln].remove(self.cursor.x);
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
