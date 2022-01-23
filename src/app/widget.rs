use crate::highlighting::Highlighter;
use crate::utils::helpers;
use crate::{
    highlighting::SyntectHighlighter,
    text_coord::TextCoord,
    text_view::{Anchor, TextView},
};
use std::collections::HashMap;
use tui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

use crate::app::InputResult;

use super::state::DacttyloGameState;

pub struct DacttyloWidget<'txt> {
    game_state: &'txt DacttyloGameState<'txt>,
}

impl<'txt> DacttyloWidget<'txt> {
    pub fn new(game_state: &'txt DacttyloGameState) -> Self {
        Self { game_state }
    }

    pub fn styles_ugh(
        &self,
        map: HashMap<usize, Option<InputResult>>,
        lines: &[&'txt str],
    ) -> HashMap<TextCoord, Option<InputResult>> {
        let mut player_tuples = map.into_iter().collect::<Vec<_>>();
        player_tuples.sort_by(|(ca, _), (cb, _)| ca.cmp(cb));
        let (indexes, inputs): (Vec<usize>, Vec<Option<InputResult>>) =
            player_tuples.into_iter().unzip();
        let coords = helpers::text_to_line_index(indexes, lines).unwrap();

        coords
            .into_iter()
            .map(|c| c.into())
            .zip(inputs)
            .collect::<HashMap<_, _>>()
    }

    pub fn get_cursor_styles(
        &self,
        lines: &[&'txt str],
    ) -> HashMap<TextCoord, tui::style::Style> {
        let state = self.game_state;

        let mut player_tuples = state
            .players()
            .iter()
            .map(|(_, pstate)| (pstate.cursor(), pstate.last_input()))
            .collect::<Vec<_>>();

        player_tuples.sort_by(|(ca, _), (cb, _)| ca.cmp(cb));
        let (indexes, inputs): (Vec<usize>, Vec<Option<InputResult>>) =
            player_tuples.into_iter().unzip();
        let coords = helpers::text_to_line_index(indexes, lines).unwrap();

        let mut player_coords = coords
            .into_iter()
            .map(Into::<TextCoord>::into)
            .zip(inputs)
            .collect::<HashMap<_, _>>();

        // Making sure the main player cursor takes precedence over the others
        let main_player = state.main_player().unwrap();
        let main_coord =
            helpers::text_to_line_index(vec![main_player.cursor()], lines)
                .unwrap()[0];
        player_coords.insert(main_coord.into(), main_player.last_input());

        let style = tui::style::Style::default();
        let neutral = style.bg(Color::White).fg(Color::Black);
        let wrong = style.bg(Color::Red).fg(Color::White);

        player_coords
            .into_iter()
            .map(|(coord, input)| {
                let style = match input {
                    Some(InputResult::Wrong(_)) => wrong,
                    _ => neutral,
                };
                (coord, style)
            })
            .collect()
    }
}

impl<'txt> Widget for DacttyloWidget<'txt> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text_lines: Vec<&str> =
            self.game_state.text().split_inclusive('\n').collect();

        let mut hl = SyntectHighlighter::new().extension("rs").build().unwrap();
        let styled_lines = hl.highlight(text_lines.as_ref());

        // let eggshell = Color::Rgb(255, 239, 214);
        // let darkblue = Color::Rgb(0, 27, 46);

        let main_player = self.game_state.main_player().unwrap();
        let main_coord = helpers::text_to_line_index(
            vec![main_player.cursor()],
            &text_lines,
        )
        .unwrap()[0];

        let view = TextView::new()
            .sparse_styling(self.get_cursor_styles(&text_lines))
            .styled_content(styled_lines)
            .anchor(Anchor::Center(main_coord.0));
        view.render(area, buf);
    }
}
