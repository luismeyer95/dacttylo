use crate::utils::helpers;
use crate::{
    text_coord::TextCoord,
    text_view::{Anchor, TextView},
};
use std::collections::HashMap;
use tui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

use crate::app::InputResult;

use super::state::DacttyloGameState;

pub struct DacttyloWidget<'txt> {
    game_state: &'txt DacttyloGameState<'txt>,
    highlighted_content: Option<Vec<Vec<(&'txt str, tui::style::Style)>>>,
}

impl<'txt> DacttyloWidget<'txt> {
    pub fn new(game_state: &'txt DacttyloGameState) -> Self {
        Self {
            game_state,
            highlighted_content: None,
        }
    }

    pub fn highlighted_content(
        mut self,
        highlighted_content: Vec<Vec<(&'txt str, tui::style::Style)>>,
    ) -> Self {
        self.highlighted_content = Some(highlighted_content);
        self
    }

    pub fn get_cursor_styles(&self) -> HashMap<TextCoord, tui::style::Style> {
        let player_coords = self.game_state.get_cursor_coords();

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
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        let text_lines: Vec<&str> =
            self.game_state.text().split_inclusive('\n').collect();

        // let eggshell = Color::Rgb(255, 239, 214);
        // let darkblue = Color::Rgb(0, 27, 46);

        let main_player = self.game_state.main_player().unwrap();
        let main_coord = helpers::text_to_line_index(
            vec![main_player.cursor()],
            &text_lines,
        )
        .unwrap()[0];

        let mut view = TextView::new()
            .sparse_styling(self.get_cursor_styles())
            .anchor(Anchor::Center(main_coord.0));

        view = match self.highlighted_content.take() {
            Some(hl_text_lines) => view.styled_content(hl_text_lines),
            None => view.content(text_lines),
        };

        view.render(area, buf);
    }
}
