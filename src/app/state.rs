#![allow(dead_code)]

use std::{collections::HashMap, error::Error};

use InputResult::*;

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

    pub fn get_progress(&self) -> Progress {
        if self.pos == self.text.len() {
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
        if pos > self.text.len() {
            Err("cursor out of bounds")
        } else {
            self.pos = pos;
            Ok(())
        }
    }

    pub fn advance_cursor(&mut self, pos: usize) -> Result<(), &'static str> {
        if pos >= self.text.len() {
            Err("cursor out of bounds")
        } else {
            self.pos += 1;
            Ok(())
        }
    }

    pub fn cursor(&self) -> usize {
        self.pos
    }

    pub fn last_input(&self) -> Option<InputResult> {
        self.last_input.clone()
    }
}

pub struct DacttyloGameState<'txt> {
    text: &'txt str,
    main_player: String,
    players: HashMap<String, PlayerState<'txt>>,
}

impl<'txt> DacttyloGameState<'txt> {
    pub fn new(main_player: &str, text: &'txt str) -> Self {
        let mut players: HashMap<String, PlayerState<'txt>> =
            Default::default();
        players.insert(main_player.to_string(), PlayerState::new(text));

        Self {
            text,
            main_player: main_player.to_string(),
            players,
        }
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

    pub fn player(&self, username: &str) -> Option<&PlayerState> {
        self.players.get(username)
    }

    pub fn main_player(&self) -> Option<&PlayerState> {
        self.players.get(&self.main_player)
    }

    pub fn players(&self) -> &HashMap<String, PlayerState<'txt>> {
        &self.players
    }

    pub fn text(&self) -> &'txt str {
        self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Progress::*;

    #[test]
    fn solo() {
        let text = "Hi";
        let mut game = DacttyloGameState::new("Luis", text);

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
        let mut game =
            DacttyloGameState::new("Luis", text).with_players(&["Agathe"]);

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
