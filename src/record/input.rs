use std::{collections::HashSet, time::Duration};

use super::elapsed::Elapsed;
use crate::app::InputResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputResultRecord {
    pub inputs: Vec<(Elapsed, InputResult)>,
}

impl InputResultRecord {
    pub fn wpm_at(&self, sampled_size: Duration, elapsed: Duration) -> f64 {
        let start = elapsed.saturating_sub(sampled_size);
        let end = elapsed;

        let sampled_correct = self
            .inputs
            .iter()
            .filter_map(|(el, ipr)| {
                let el: Duration = el.clone().into();
                if el >= start && el < end {
                    Some(ipr)
                } else {
                    None
                }
            })
            .filter(|ipr| matches!(ipr, InputResult::Correct(_)))
            .count() as u32;

        let cps = f64::from(sampled_correct) / sampled_size.as_secs_f64();
        cps * 60.0 / 5.0
    }

    pub fn count_correct(&self) -> usize {
        self.inputs
            .iter()
            .filter(|(_, ipr)| matches!(ipr, InputResult::Correct(_)))
            .count()
    }

    pub fn count_wrong(&self) -> usize {
        self.inputs
            .iter()
            .filter(|(_, ipr)| matches!(ipr, InputResult::Wrong(_)))
            .count()
    }

    pub fn average_wpm(&self) -> f64 {
        let last_ipr = self.inputs.iter().rev().next();

        match last_ipr {
            Some((elapsed, _)) => {
                let elapsed_seconds =
                    Into::<Duration>::into(elapsed.clone()).as_secs_f64();
                let total_correct = self.count_correct();

                let cps = total_correct as f64 / elapsed_seconds;
                cps * 60.0 / 5.0
            }
            None => 0.0,
        }
    }

    pub fn top_wpm(&self, sampled_size: Duration, step: Duration) -> f64 {
        let last_ipr = self.inputs.iter().rev().next();

        match last_ipr {
            Some((elapsed, _)) => {
                let elapsed_seconds = Into::<Duration>::into(elapsed.clone());
                let mut current_elapsed = Duration::ZERO;
                let mut max_wpm = 0.0;

                while current_elapsed < elapsed_seconds {
                    max_wpm = f64::max(
                        max_wpm,
                        self.wpm_at(sampled_size, current_elapsed),
                    );
                    current_elapsed += step;
                }

                max_wpm
            }
            None => 0.0,
        }
    }

    pub fn mistake_stats(&self) -> Vec<(char, usize)> {
        let mut mistake_charlist = self
            .inputs
            .iter()
            .filter_map(|(_, ipr)| match ipr {
                InputResult::Wrong(c) => Some(*c),
                _ => None,
            })
            .collect::<Vec<char>>();

        let mistake_charset =
            HashSet::<_>::from_iter(mistake_charlist.iter().copied());

        mistake_charset
            .into_iter()
            .map(|char| {
                let (char_stat, rest) =
                    mistake_charlist.iter().partition(|&&c| c == char);
                mistake_charlist = rest;
                (char, char_stat.len())
            })
            .collect()
    }

    pub fn precision(&self) -> f64 {
        self.count_correct() as f64 / self.inputs.len() as f64
    }
}

impl From<Vec<(Elapsed, InputResult)>> for InputResultRecord {
    fn from(v: Vec<(Elapsed, InputResult)>) -> Self {
        InputResultRecord { inputs: v }
    }
}

impl From<InputResultRecord> for Vec<(Elapsed, InputResult)> {
    fn from(val: InputResultRecord) -> Self {
        val.inputs
    }
}
