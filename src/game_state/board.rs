use crossterm::cursor;
use itertools::Itertools;
use once_cell::sync::OnceCell;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::env::current_dir;
use std::error::Error;
use std::io::BufRead;
use std::iter::Peekable;
use std::ops::{Not, Range};
use std::str::CharIndices;
use std::str::FromStr;
use syntect::easy::{HighlightFile, HighlightLines};
use syntect::highlighting::{self, FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

mod style;
use style::Style;
type SyntectStyle = syntect::highlighting::Style;
type TuiStyle = tui::style::Style;
type Color = tui::style::Color;

#[derive(Clone, Default)]
struct Cursor {
    pub index: usize,
    pub style: Style,
    pub precedence: u8,
}

#[derive(Debug)]
struct TokenMarker {
    range: Range<usize>,
    style: Style,
}

#[derive(Default)]
struct Board {
    text: String,
    tokens: Vec<TokenMarker>,
    cursors: HashMap<String, Cursor>,
}

struct BoardIter<'a> {
    board: &'a Board,
    display_cursors: Vec<&'a Cursor>,

    text_iter: CharIndices<'a>,
    token_idx: usize,
    cursor_idx: usize,
}

impl<'a> BoardIter<'a> {
    fn resolve_displayed_cursors(cursors: &'a HashMap<String, Cursor>) -> Vec<&'a Cursor> {
        let mut precedence_map: BTreeMap<usize, &'a Cursor> = BTreeMap::new();
        for (_, c) in cursors {
            let v = precedence_map.entry(c.index).or_insert_with(|| c);
            if c.precedence > v.precedence {
                precedence_map.insert(c.index, c);
            }
        }
        precedence_map.into_iter().map(|(_, c)| c).collect()
    }

    fn resolve_token_style(&mut self, index: usize, board: &'a Board) -> &'a Style {
        let mut tkn = board.tokens.get(self.token_idx).unwrap();
        if tkn.range.contains(&index).not() {
            self.token_idx += 1;
            tkn = board.tokens.get(self.token_idx).unwrap();
        }
        &tkn.style
    }

    pub fn new(board: &'a Board) -> BoardIter<'a> {
        let display_cursors = Self::resolve_displayed_cursors(&board.cursors);
        Self {
            board,
            display_cursors,
            text_iter: board.text.char_indices(),
            cursor_idx: 0,
            token_idx: 0,
        }
    }
}

impl<'a> Iterator for BoardIter<'a> {
    type Item = (char, &'a Style);

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        let (i, ch) = self.text_iter.next()?;
        let tkn = self.resolve_token_style(i, self.board);

        let item = match self.display_cursors.get(self.cursor_idx) {
            Some(&cursor) => (cursor.index == i)
                .then(|| self.cursor_idx += 1)
                .map_or_else(|| (ch, tkn), |_| (ch, &cursor.style)),
            None => (ch, tkn),
        };
        Some(item)
    }
}

impl Board {
    pub fn new(file_path: &str) -> Result<Board, Box<dyn Error>> {
        let (syntax_set, theme_set) = Self::load_defaults();
        let syntax = syntax_set
            .find_syntax_for_file(file_path)
            .unwrap()
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

        let text = std::fs::read_to_string(std::path::Path::new(file_path))?;
        let mut highlighter = HighlightLines::new(syntax, &theme_set.themes["base16-ocean.dark"]);
        let mut token_marks = Self::tokenize_text(&text, highlighter);

        Ok(Board {
            text,
            tokens: token_marks,
            cursors: HashMap::new(),
        })
    }

    pub fn from_str(text: &str, syntax_ext: &str) -> Result<Board, Box<dyn Error>> {
        let (syntax_set, theme_set) = Self::load_defaults();
        let syntax = syntax_set
            .find_syntax_by_extension(syntax_ext)
            .expect("syntax extension not found");

        let mut highlighter = HighlightLines::new(syntax, &theme_set.themes["base16-ocean.dark"]);
        let mut token_marks = Self::tokenize_text(text, highlighter);

        Ok(Board {
            text: text.to_string(),
            tokens: token_marks,
            cursors: HashMap::new(),
        })
    }

    pub fn iter(&self) -> BoardIter {
        BoardIter::new(self)
    }

    pub fn iter_token(&self) -> BoardIter {
        let mut prev_style: Option<Style> = None;
        let mut buffer: &str = &"";
        // BoardIter::new(self).map(|(ch, style)| match prev_style {
        //     Some(prev_style) => {
        //         if (style == prev_style) {
        //         } else {
        //         }
        //     }
        //     None => (),
        // });
        todo!()
    }

    pub fn get_cursor(&mut self, key: &str) -> Entry<String, Cursor> {
        self.cursors.entry(key.to_string())
    }

    pub fn remove_cursor(&mut self, key: &str) {
        self.cursors.remove(key);
    }

    fn load_defaults() -> (&'static SyntaxSet, &'static ThemeSet) {
        static SYNTAX_SET: OnceCell<SyntaxSet> = OnceCell::new();
        static THEME_SET: OnceCell<ThemeSet> = OnceCell::new();
        (
            SYNTAX_SET.get_or_init(|| SyntaxSet::load_defaults_newlines()),
            THEME_SET.get_or_init(|| ThemeSet::load_defaults()),
        )
    }

    fn tokenize_text(text: &str, mut highlighter: HighlightLines) -> Vec<TokenMarker> {
        let (syntax_set, theme_set) = Self::load_defaults();

        let mut tokenized_contents: Vec<(SyntectStyle, &str)> = vec![];
        for line in LinesWithEndings::from(&text) {
            let mut tokens: Vec<(SyntectStyle, &str)> = highlighter.highlight(&line, &syntax_set);
            tokenized_contents.extend(tokens);
        }

        Self::to_token_markers(tokenized_contents)
    }

    fn to_token_markers(line: Vec<(SyntectStyle, &str)>) -> Vec<TokenMarker> {
        let mut result: Vec<TokenMarker> = vec![];
        let mut accumulated_index: usize = 0;
        for (style, slice) in line {
            let tk_marker = TokenMarker {
                range: accumulated_index..accumulated_index + slice.len(),
                style: style.into(),
            };
            accumulated_index += slice.len();
            result.push(tk_marker);
        }
        result
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn iter_test() {
        let mut b = Board::new("file.rs").unwrap();

        let luis_cursor = Cursor {
            index: 2,
            style: TuiStyle::default().fg(Color::Red).bg(Color::White).into(),
            precedence: 0,
        };

        let agathe_cursor = Cursor {
            index: 1,
            style: TuiStyle::default().fg(Color::Blue).bg(Color::White).into(),
            precedence: 1,
        };

        // b.set_cursor("luis", cursor)
        let l = b.get_cursor("luis").or_insert(luis_cursor);
        let a = b.get_cursor("agathe").or_insert(agathe_cursor);

        for tkn in b.iter() {
            println!("{:?} {:?}", tkn.0, tkn.1.foreground);
        }
    }

    #[test]
    fn board_test() {
        let mut b = Board::new("file.rs").unwrap();

        b.tokens
            .iter()
            .for_each(|el| println!("{:?} {:?}", &b.text[el.range.clone()], el.style.foreground));
    }
}
