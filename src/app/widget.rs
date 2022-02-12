use crate::utils::helpers;
use crate::{
    text_coord::TextCoord,
    text_view::{Anchor, TextView},
};
use std::collections::HashMap;
use std::iter;
use tui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

use crate::app::InputResult;

use super::state::{PlayerPool, PlayerState};

pub struct DacttyloWidget<'txt> {
    main: &'txt PlayerState<'txt>,
    opponents: &'txt PlayerPool<'txt>,

    highlighted_content: Option<Vec<Vec<(&'txt str, tui::style::Style)>>>,
}

impl<'txt> DacttyloWidget<'txt> {
    pub fn new(main: &'txt PlayerState, opponents: &'txt PlayerPool) -> Self {
        Self {
            main,
            opponents,
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

    fn get_main_style(&self) -> (TextCoord, tui::style::Style) {
        let player_coords = self.main.get_cursor_coord();

        let style = tui::style::Style::default();
        let neutral = style.bg(Color::White).fg(Color::Black);
        let wrong = style.bg(Color::Red).fg(Color::White);

        let style = match self.main.last_input() {
            Some(InputResult::Wrong(_)) => wrong,
            _ => neutral,
        };

        (player_coords, style)
    }

    fn get_opponent_styles(&self) -> HashMap<TextCoord, tui::style::Style> {
        let opponent_coords = self.opponents.get_cursor_coords();

        let style = tui::style::Style::default();
        let grey = style.bg(Color::Rgb(20, 20, 20)).fg(Color::White);

        opponent_coords
            .into_iter()
            .map(|(coord, _)| (coord, grey))
            .collect()
    }
}

impl<'txt> Widget for DacttyloWidget<'txt> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        let text_lines: Vec<&str> =
            self.opponents.text().split_inclusive('\n').collect();

        let main_coord = self.main.get_cursor_coord();

        let mut styles = self.get_opponent_styles();
        let main_style = self.get_main_style();
        styles.extend(HashMap::<_, _>::from_iter(iter::once(main_style)));

        // let eggshell = Color::Rgb(255, 239, 214);
        // let darkblue = Color::Rgb(0, 27, 46);

        let mut view = TextView::new()
            .sparse_styling(styles)
            .anchor(Anchor::Center(main_coord.ln));

        view = match self.highlighted_content.take() {
            Some(hl_text_lines) => view.styled_content(hl_text_lines),
            None => view.content(text_lines),
        };

        view.render(area, buf);
    }
}
