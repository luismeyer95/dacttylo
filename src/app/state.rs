#![allow(dead_code)]

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use InputResult::*;

use crate::{
    record::recorder::InputResultRecorder,
    text_coord::TextCoord,
    utils::{helpers::text_to_line_index, types::AsyncResult},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputResult {
    Correct,
    Wrong(char),
}

pub struct PlayerState<'txt> {
    pub name: String,
    pub recorder: InputResultRecorder,

    text: &'txt str,
    pos: usize,
    max_pos: usize,

    errors: BTreeSet<usize>,
}

impl<'txt> PlayerState<'txt> {
    pub fn new(name: String, text: &'txt str) -> Self {
        Self {
            name,
            text,
            pos: 0,
            max_pos: text.chars().count(),
            errors: BTreeSet::new(),
            recorder: InputResultRecorder::new(),
        }
    }

    pub fn process_input(&mut self, input_ch: char) -> Option<InputResult> {
        let cursor_ch = self.text.chars().nth(self.pos)?;

        let input_result = if input_ch == cursor_ch {
            self.pos += 1;

            // TODO: take whitespace autoskip into account for input recording

            // if cursor_ch == '\n' {
            //     self.skip_trailing_wp();
            // }
            Correct
        } else {
            self.errors.insert(self.pos);
            Wrong(cursor_ch)
        };

        self.recorder.push(input_result);
        Some(input_result)
    }

    fn skip_trailing_wp(&mut self) {
        let it = self.text.chars().skip(self.pos);
        for ch in it {
            if !ch.is_whitespace() || ch == '\n' {
                break;
            }
            self.pos += 1;
        }
    }

    pub fn get_error_coords(&self) -> Vec<TextCoord> {
        let text_lines = self.text.split_inclusive('\n').collect::<Vec<_>>();
        let errors: Vec<usize> = Vec::from_iter(self.errors.clone());
        let coords = text_to_line_index(errors, &text_lines).unwrap();

        coords.into_iter().map_into::<TextCoord>().collect()
    }

    pub fn get_cursor_coord(&self) -> TextCoord {
        let text_lines = self.text.split_inclusive('\n').collect::<Vec<_>>();
        let coords_lst = text_to_line_index([self.pos], &text_lines).unwrap();

        coords_lst[0].into()
    }

    pub fn is_done(&self) -> bool {
        self.pos == self.max_pos
    }

    pub fn set_cursor(&mut self, pos: usize) -> Result<(), &'static str> {
        if pos > self.text.chars().count() {
            Err("cursor out of bounds")
        } else {
            self.pos = pos;
            Ok(())
        }
    }

    pub fn advance_cursor(&mut self) -> Result<(), &'static str> {
        self.set_cursor(self.pos + 1)
    }

    pub fn cursor(&self) -> usize {
        self.pos
    }

    pub fn last_input(&self) -> Option<InputResult> {
        self.recorder
            .record()
            .inputs
            .last()
            .map(|(elapsed, input)| input.clone())
    }

    pub fn text(&self) -> &str {
        self.text
    }
}

pub struct PlayerPool<'txt> {
    text: &'txt str,

    players: HashMap<String, PlayerState<'txt>>,
}

impl<'txt> PlayerPool<'txt> {
    pub fn new(text: &'txt str) -> Self {
        let players: HashMap<String, PlayerState<'txt>> = Default::default();

        Self { text, players }
    }

    pub fn with_players(mut self, usernames: &[&str]) -> Self {
        for &user in usernames {
            let username = user.to_string();
            self.players
                .entry(username.clone())
                .or_insert_with(|| PlayerState::new(username, self.text));
        }

        self
    }

    pub fn process_input(
        &mut self,
        username: &str,
        input_ch: char,
    ) -> AsyncResult<InputResult> {
        let player = self
            .players
            .get_mut(username)
            .ok_or("Player does not exist")?;

        let input_result = player
            .process_input(input_ch)
            .ok_or("Played already reached the end")?;

        Ok(input_result)
    }

    pub fn advance_player(&mut self, username: &str) -> AsyncResult<()> {
        let player = self
            .players
            .get_mut(username)
            .ok_or("Player does not exist")?;

        player.advance_cursor()?;

        Ok(())
    }

    pub fn player(&self, username: &str) -> Option<&PlayerState> {
        self.players.get(username)
    }

    pub fn players(&self) -> &HashMap<String, PlayerState<'txt>> {
        &self.players
    }

    pub fn text(&self) -> &'txt str {
        self.text
    }

    pub fn are_done(&self) -> bool {
        self.players
            .iter()
            .map(|(_, state)| state.is_done())
            .all(|done| done)
    }

    pub fn get_cursor_coords(&self) -> HashMap<TextCoord, Option<InputResult>> {
        let text_lines = self.text.split_inclusive('\n').collect::<Vec<_>>();

        let mut player_tuples = self
            .players()
            .iter()
            .filter_map(|(_, pstate)| {
                if pstate.cursor() < self.text.len() {
                    Some((pstate.cursor(), pstate.last_input()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        player_tuples.sort_by(|(ca, _), (cb, _)| ca.cmp(cb));
        let (indexes, inputs): (Vec<usize>, Vec<Option<InputResult>>) =
            player_tuples.into_iter().unzip();
        let coords = text_to_line_index(indexes, &text_lines).unwrap();

        coords
            .into_iter()
            .map(Into::<TextCoord>::into)
            .zip(inputs)
            .collect::<HashMap<_, _>>()
    }
}
