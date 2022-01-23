use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use unicode_segmentation::UnicodeSegmentation;

pub fn is_valid_file(val: &str) -> Result<(), String> {
    if std::path::Path::new(val).exists() {
        Ok(())
    } else {
        Err(format!("file `{}` does not exist.", val))
    }
}

pub fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension().and_then(|s| s.to_str())
}

pub fn input_width(s: &str) -> usize {
    UnicodeSegmentation::graphemes(s, true).count()
}

pub fn is_sorted<I>(data: I) -> bool
where
    I: IntoIterator,
    I::Item: Ord + Clone,
{
    data.into_iter().tuple_windows().all(|(a, b)| a <= b)
}

/// Converts a list of 1D text buffer position into a vector of tuples containing
/// line number and a character index into that line
pub fn text_to_line_index(
    // /!\ Assumes sorted indexes /!\
    indexes: impl Into<Vec<usize>>,
    text_lines: &[&str],
) -> Result<Vec<(usize, usize)>, &'static str> {
    let mut indexes: Vec<usize> = indexes.into();
    if !is_sorted(&indexes) {
        return Err("indexes not sorted");
    }

    let size = indexes.len();
    let mut coords: Vec<(usize, usize)> = vec![];

    for (i, &line) in text_lines.iter().enumerate() {
        let ln_width = input_width(line);
        let (matched, remainder): (Vec<usize>, Vec<usize>) =
            indexes.into_iter().partition(|&idx| idx < ln_width);
        coords.extend(matched.into_iter().map(|idx| (i, idx)));
        if coords.len() == size {
            return Ok(coords);
        }
        indexes = remainder.into_iter().map(|idx| idx - ln_width).collect();
    }
    Err("index out of bounds")
}

pub fn line_to_text_index(
    ln_index: usize,
    text_lines: Vec<&str>,
) -> Result<usize, &'static str> {
    if ln_index > text_lines.len() {
        Err("index out of bounds")
    } else {
        Ok(text_lines
            .into_iter()
            .enumerate()
            .take_while(|(i, _)| i != &ln_index)
            .fold(0, |acc, (_, el)| acc + input_width(el)))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    #[test]
    fn line_starts_ttli() {
        let lines: Vec<&str> =
            vec!["Hello how are", "you today my", "good sir"];
        let indexes = [0, 13, 25];

        let result = text_to_line_index(indexes, &lines).unwrap();

        assert_eq!(result, vec![(0, 0), (1, 0), (2, 0)]);
    }

    #[test]
    fn edges_ttli() {
        let lines: Vec<&str> =
            vec!["Hello how are", "you today my", "good sir"];
        let indexes = [12, 24, 32];

        let result = text_to_line_index(indexes, &lines).unwrap();

        assert_eq!(result, vec![(0, 12), (1, 11), (2, 7)]);
    }

    #[test]
    fn full_ttli() {
        let lines: Vec<&str> =
            vec!["Hello how are", "you today my", "good sir"];
        let indexes = lines
            .iter()
            .flat_map(|s| s.chars())
            .enumerate()
            .map(|(i, _)| i)
            .collect::<Vec<_>>();

        let mut result: VecDeque<_> = text_to_line_index(indexes, &lines)
            .unwrap()
            .into_iter()
            .collect();

        println!("{:?}", result);

        for (lni, &ln) in lines.iter().enumerate() {
            for (chi, _) in UnicodeSegmentation::graphemes(ln, true).enumerate()
            {
                assert_eq!(result.pop_front(), Some((lni, chi)));
            }
        }
    }
}
