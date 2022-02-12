#![allow(dead_code)]

use std::{collections::HashMap, error::Error};

use InputResult::*;

use crate::{text_coord::TextCoord, utils::helpers};

#[derive(Debug, Clone, PartialEq)]
pub enum Progress {
    Ongoing,
    Finished,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputResult {
    Correct(Progress),
    Wrong(char),
}

pub struct PlayerState<'txt> {
    text: &'txt str,
    pos: usize,
    last_input: Option<InputResult>,
}

impl<'txt> PlayerState<'txt> {
    pub fn new(text: &'txt str) -> Self {
        Self {
            pos: 0,
            text,
            last_input: None,
        }
    }

    pub fn get_cursor_coord(&self) -> TextCoord {
        let text_lines = self.text.split_inclusive('\n').collect::<Vec<_>>();
        let coords_lst =
            helpers::text_to_line_index([self.pos], &text_lines).unwrap();
        coords_lst[0].into()
    }

    pub fn get_progress(&self) -> Progress {
        if self.pos == self.text.chars().count() {
            Progress::Finished
        } else {
            Progress::Ongoing
        }
    }

    pub fn process_input(&mut self, input_ch: char) -> Option<InputResult> {
        let cursor_ch = self.text.chars().nth(self.pos)?;

        if input_ch == cursor_ch {
            self.pos += 1;
            self.last_input = Some(Correct(self.get_progress()));
        } else {
            self.last_input = Some(Wrong(cursor_ch));
        }

        self.last_input.clone()
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
        self.last_input.clone()
    }
}

pub struct PlayerPool<'txt> {
    text: &'txt str,

    players: HashMap<String, PlayerState<'txt>>,
}

impl<'txt> PlayerPool<'txt> {
    pub fn new(text: &'txt str) -> Self {
        let mut players: HashMap<String, PlayerState<'txt>> =
            Default::default();

        Self { text, players }
    }

    pub fn with_players(mut self, usernames: &[&str]) -> Self {
        for &user in usernames {
            self.players
                .entry(user.to_string())
                .or_insert_with(|| PlayerState::new(self.text));
        }

        self
    }

    pub fn process_input(
        &mut self,
        username: &str,
        input_ch: char,
    ) -> Result<InputResult, Box<dyn Error>> {
        let player = self
            .players
            .get_mut(username)
            .ok_or("Player does not exist")?;

        let input_result = player
            .process_input(input_ch)
            .ok_or("Played already reached the end")?;

        Ok(input_result)
    }

    pub fn advance_player(
        &mut self,
        username: &str,
    ) -> Result<(), Box<dyn Error>> {
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

    pub fn get_cursor_coords(&self) -> HashMap<TextCoord, Option<InputResult>> {
        let text_lines = self.text.split_inclusive('\n').collect::<Vec<_>>();

        let mut player_tuples = self
            .players()
            .iter()
            .map(|(_, pstate)| (pstate.cursor(), pstate.last_input()))
            .collect::<Vec<_>>();

        player_tuples.sort_by(|(ca, _), (cb, _)| ca.cmp(cb));
        let (indexes, inputs): (Vec<usize>, Vec<Option<InputResult>>) =
            player_tuples.into_iter().unzip();
        let coords = helpers::text_to_line_index(indexes, &text_lines).unwrap();

        let mut player_coords = coords
            .into_iter()
            .map(Into::<TextCoord>::into)
            .zip(inputs)
            .collect::<HashMap<_, _>>();

        player_coords
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Progress::*;

    #[test]
    fn solo() {
        let text = "Hi";
        let mut game = PlayerPool::new(text);

        assert_eq!(
            game.process_input("Luis", 'H').unwrap(),
            InputResult::Correct(Ongoing)
        );

        assert_eq!(
            game.process_input("Luis", 'o').unwrap(),
            InputResult::Wrong('i')
        );

        assert_eq!(
            game.process_input("Luis", 'i').unwrap(),
            InputResult::Correct(Finished)
        );
    }

    #[test]
    fn multi() {
        let text = "Hi";
        let mut game = PlayerPool::new(text).with_players(&["Agathe"]);

        assert_eq!(
            game.process_input("Luis", 'H').unwrap(),
            InputResult::Correct(Ongoing)
        );

        assert_eq!(
            game.process_input("Agathe", 'Y').unwrap(),
            InputResult::Wrong('H')
        );

        assert_eq!(
            game.process_input("Luis", 'i').unwrap(),
            InputResult::Correct(Finished)
        );
    }
}
