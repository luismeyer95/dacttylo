#![feature(iter_intersperse)]

use once_cell::sync::OnceCell;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::env::current_dir;
use std::error::Error;
use std::io::BufRead;
use std::ops::{Not, Range};
use syntect::easy::{HighlightFile, HighlightLines};
use syntect::highlighting::{self, Color, FontStyle, Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

fn load_defaults() -> (&'static SyntaxSet, &'static ThemeSet) {
    static SYNTAX_SET: OnceCell<SyntaxSet> = OnceCell::new();
    static THEME_SET: OnceCell<ThemeSet> = OnceCell::new();
    (
        SYNTAX_SET.get_or_init(|| SyntaxSet::load_defaults_newlines()),
        THEME_SET.get_or_init(|| ThemeSet::load_defaults()),
    )
}

fn file_to_string(s: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(std::path::Path::new(s))
}

fn split_token_bytes((style, token): (Style, &str)) -> Vec<(Style, &str)> {
    token
        .chars()
        .enumerate()
        .map(|(i, _)| (style.clone(), &token[i..i + 1]))
        .collect::<Vec<(Style, &str)>>()
}

fn to_token_markers(line: Vec<(Style, &str)>) -> Vec<TokenMarker> {
    line.iter()
        .map(|token_tup| {
            if token_tup.1.contains('\t') {
                split_token_bytes(*token_tup)
            } else {
                vec![*token_tup]
            }
        })
        .flatten()
        .scan(0, |idx, (style, slice)| {
            let tkref = TokenMarker {
                range: *idx..*idx + slice.len(),
                style,
            };
            *idx += slice.len();
            Some(tkref)
        })
        .collect::<Vec<TokenMarker>>()
}

#[derive(Clone)]
struct Cursor {
    index: usize,
    style: Style,
    precedence: u8,
}

#[derive(Debug)]
struct TokenMarker {
    range: Range<usize>,
    style: Style,
}

struct Board {
    text: String,
    tokens: Vec<TokenMarker>,
    cursors: Vec<Cursor>,
}

struct BoardIter<'a> {
    // text: &'a str,
    board: &'a Board,
    // tokens: Vec<(&'a TokenMarker)>, // cur_text_index: usize,
    token_idx: usize,
    cursor_idx: usize,
    queue: VecDeque<(&'a str, Style)>,
}

impl<'a> BoardIter<'a> {
    pub fn new(board: &'a Board) -> Self {
        if !Self::cursors_are_sorted(&board.cursors) {
            panic!("board iterator logic depends on cursors being sorted by index");
        }
        Self {
            board,
            cursor_idx: 0,
            token_idx: 0,
            queue: VecDeque::<_>::default(),
        }
    }

    fn cursors_are_sorted(cursors: &[Cursor]) -> bool {
        cursors.windows(2).all(|p| p[0].index <= p[1].index)
    }

    fn enqueue_token(&mut self, token: &TokenMarker) {
        let slice = &self.board.text[token.range.clone()];
        self.queue.push_back((slice, token.style));
    }

    fn split_enqueue(&mut self, token: &TokenMarker, cursors: &[&Cursor]) {
        let mut cursor_idx = 0;
        let mut slice_start = token.range.start;
        for i in token.range.clone() {
            if let Some(&cursor) = cursors.get(cursor_idx) {
                if slice_start < i {
                    let tkn: (&str, Style) = (&self.board.text[slice_start..i], token.style);
                    self.queue.push_back(tkn);
                }
                self.queue
                    .push_back((&self.board.text[i..i + 1], cursor.style));
                slice_start = i + 1;
                cursor_idx += 1;
            }
        }
        if slice_start < token.range.end {
            self.queue
                .push_back((&self.board.text[slice_start..token.range.end], token.style));
        };
    }

    fn resolve_cursor_precedence(cursors: Vec<&Cursor>) -> Vec<&Cursor> {
        let mut precedence_map: BTreeMap<usize, &Cursor> = BTreeMap::new();
        for c in cursors {
            let v = precedence_map.entry(c.index).or_insert(c);
            if c.precedence > v.precedence {
                precedence_map.insert(c.index, c);
            }
        }
        precedence_map
            .into_iter()
            .map(|(_, c)| c)
            .collect::<Vec<&Cursor>>()
    }

    fn pop_cursors_in_range<'b>(
        cursors: &'b Vec<Cursor>,
        cursor_idx: &mut usize,
        token: &TokenMarker,
    ) -> Option<Vec<&'b Cursor>> {
        let cursors_slice = cursors.get(*cursor_idx..)?;

        let cursors_in_range = cursors_slice
            .iter()
            .take_while(|&c| token.range.contains(&c.index))
            .collect::<Vec<&Cursor>>();
        *cursor_idx += cursors_in_range.len();

        cursors_in_range
            .is_empty()
            .not()
            .then(|| Self::resolve_cursor_precedence(cursors_in_range))
    }

    fn process_token(&mut self, token: &TokenMarker) {
        match Self::pop_cursors_in_range(&self.board.cursors, &mut self.cursor_idx, token) {
            Some(cursors) => self.split_enqueue(token, &cursors),
            None => self.enqueue_token(token),
        };
    }
}

impl<'a> Iterator for BoardIter<'a> {
    type Item = (&'a str, Style);

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if let Some(queued_item) = self.queue.pop_front() {
            return Some(queued_item);
        } else if let Some(token) = self.board.tokens.get(self.token_idx) {
            self.process_token(token);
        }

        self.queue.pop_front()
    }
}

impl Board {
    pub fn new(file_path: &str) -> Result<Board, Box<dyn Error>> {
        let (syntax_set, theme_set) = load_defaults();
        let syntax = syntax_set
            .find_syntax_for_file(file_path)
            .unwrap()
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

        let text = file_to_string(file_path).unwrap();
        let mut highlighter = HighlightLines::new(syntax, &theme_set.themes["base16-ocean.dark"]);

        let mut tokenized_contents: Vec<TokenMarker> = vec![];
        for line in LinesWithEndings::from(&text) {
            let mut tokens: Vec<(Style, &str)> = highlighter.highlight(&line, &syntax_set);
            let mut tokens = to_token_markers(tokens);
            tokenized_contents.extend(tokens);
        }

        Ok(Board {
            text,
            tokens: tokenized_contents,
            cursors: vec![],
        })
    }

    pub fn iter(&self) -> BoardIter {
        BoardIter::new(self)
    }
}
