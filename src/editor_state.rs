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
                self.cursor.x = min(self.cursor.x, self.text_lines[self.cursor.ln].len() - 1);
            }
            Cursor::Down => {
                self.cursor.ln = min(self.cursor.ln.saturating_add(1), self.text_lines.len() - 1);
                self.cursor.x = min(self.cursor.x, self.text_lines[self.cursor.ln].len() - 1);
            }
            Cursor::Left => self.cursor.x = self.cursor.x.saturating_sub(1),
            Cursor::Right => {
                let ln = &mut self.text_lines[self.cursor.ln];
                match cursor_ch {
                    Some('\n') | None => {}
                    Some(_) => {
                        self.cursor.x = min(self.cursor.x.saturating_add(1), ln.len());
                    }
                }
            }
        }
    }

    fn char_at(&self, coord: TextCoord) -> Option<char> {
        self.text_lines
            .get(coord.ln)
            .and_then(|ln| ln.chars().nth(coord.x))
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
        loop {
            let ln_len = self.text_lines[coord.ln].len();
            let cutoff = min(coord.x.saturating_add(offset), ln_len);
            offset = offset.saturating_sub(cutoff - coord.x);
            coord.x = cutoff;
            if offset == 0 {
                return Some(coord);
            }
            let next_ln = coord.ln.checked_add(1)?;
            if next_ln >= self.text_lines.len() {
                return None;
            }
            coord.ln = next_ln;
            coord.x = 0;
        }
    }

    fn offset_neg(&self, mut offset: usize, mut coord: TextCoord) -> Option<TextCoord> {
        loop {
            let cutoff = coord.x.saturating_sub(offset);
            offset = offset.saturating_sub(coord.x - cutoff);
            coord.x = cutoff;
            if offset == 0 {
                return Some(coord);
            }
            coord.ln = coord.ln.checked_sub(1)?;
            coord.x = self.text_lines[coord.ln].len();
        }
    }

    pub fn set_cursor_ln_start(&mut self) {
        self.cursor.ln = 0;
    }

    pub fn set_cursor_ln_end(&mut self) {
        self.cursor.ln = self.text_lines.len();
    }

    pub fn set_cursor_x_start(&mut self) {
        self.cursor.x = 0;
    }

    pub fn set_cursor_x_end(&mut self) {
        self.cursor.x = self.text_lines[self.cursor.ln].len();
    }

    pub fn insert_ch(&mut self, c: char) {
        let ln = &mut self.text_lines[self.cursor.ln];
        ln.insert(self.cursor.x, c);
    }

    pub fn delete_ch(&mut self) {
        let c = self.cursor_ch();
        // error-prone... order matters
        let mut text_lines = std::mem::take(&mut self.text_lines);
        match c {
            Some('\n') => {
                // assumes presence of newline guarantees this line isn't the last
                let ln_below = text_lines.remove(self.cursor.ln + 1);
                text_lines[self.cursor.ln].replace_range(self.cursor.x.., &ln_below);
            }
            Some(_) => {
                text_lines[self.cursor.ln].remove(self.cursor.x);
            }
            None => {}
        };
        self.text_lines = text_lines;
    }

    pub fn cursor_ch(&self) -> Option<char> {
        let ln = self.text_lines.get(self.cursor.ln)?;
        let c = ln.chars().nth(self.cursor.x);
        c
    }

    pub fn insert_ln(&mut self) {
        let ln = &mut self.text_lines[self.cursor.ln];
        let x = self.cursor.x;
        let carry = ln[x..].to_string();
        ln.replace_range(x.., "");
        self.text_lines.insert(self.cursor.ln + 1, carry);
    }

    pub fn get_cursor(&self) -> TextCoord {
        let ln = &self.text_lines[self.cursor.ln];
        TextCoord::new(self.cursor.ln, self.cursor.x)
    }

    pub fn get_lines(&self) -> Vec<&str> {
        self.text_lines.iter().map(|s| s.as_str()).collect()
    }
}
