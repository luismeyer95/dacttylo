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
        match cmd {
            Cursor::Up => {
                self.cursor.ln = self.cursor.ln.saturating_sub(1);
                self.cursor.x = min(self.cursor.x, self.text_lines[self.cursor.ln].len());
            }
            Cursor::Down => {
                self.cursor.ln = min(self.cursor.ln.saturating_add(1), self.text_lines.len());
                self.cursor.x = min(self.cursor.x, self.text_lines[self.cursor.ln].len());
            }
            Cursor::Left => self.cursor.x = self.cursor.x.saturating_sub(1),
            Cursor::Right => {
                let ln = &mut self.text_lines[self.cursor.ln];
                self.cursor.x = min(self.cursor.x.saturating_add(1), ln.len());
            }
        }
    }

    fn char_at(&self, coord: TextCoord) -> Option<char> {
        self.text_lines
            .get(coord.ln)
            .and_then(|ln| ln.chars().nth(coord.x))
    }

    fn flat_offset(&self, offset: isize, mut coord: TextCoord) -> Option<TextCoord> {
        if offset < 0 {
            let mut left_offset = offset.unsigned_abs();
            loop {
                coord.x = coord.x.saturating_sub(left_offset);
                left_offset = left_offset.saturating_sub(coord.x);
                if left_offset == 0 {
                    return Some(coord);
                }
                coord.ln.checked_sub(1)?;
                coord.x = self.text_lines[coord.ln].len() - 1;
            }
        } else {
        }
        todo!();
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
        let ln = &mut self.text_lines[self.cursor.ln];
        if ln.len() != 0 {
            ln.remove(self.cursor.x);
        } else {
            if self.text_lines.len() == 1 {
                return;
            }
            self.text_lines.remove(self.cursor.ln);
            if (0..self.text_lines[self.cursor.ln].len())
                .contains(&self.cursor.ln)
                .not()
            {}
        }
    }

    pub fn cursor_ch(&self) -> Option<char> {
        let ln = &self.text_lines[self.cursor.ln];
        ln.chars().nth(self.cursor.x)
    }

    pub fn insert_ln(&mut self) {
        let ln = &mut self.text_lines[self.cursor.ln];
        let x = self.cursor.x;
        let carry = ln[x..].to_string();
        ln.replace_range(x.., "\n");
        self.text_lines.insert(self.cursor.ln + 1, carry);
        self.set_cursor_x_start();
    }

    pub fn get_cursor(&self) -> TextCoord {
        let ln = &self.text_lines[self.cursor.ln];
        TextCoord::new(self.cursor.ln, self.cursor.x)
    }

    pub fn get_lines(&self) -> Vec<&str> {
        self.text_lines.iter().map(|s| s.as_str()).collect()
    }
}
