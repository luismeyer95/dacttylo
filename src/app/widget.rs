use crate::{
    text_coord::TextCoord,
    text_view::{Anchor, TextView},
    utils::types::StyledLine,
};
use std::collections::HashMap;
use tui::style::Style;
use tui::text::StyledGrapheme;
use tui::widgets::Block;
use tui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

use crate::app::InputResult;

use super::state::{PlayerPool, PlayerState};

pub struct DacttyloWidget<'txt, 'ln> {
    block: Block<'txt>,

    main: &'txt PlayerState<'txt>,
    opponents: &'txt PlayerPool<'txt>,

    highlighted_content: &'ln [StyledLine<'txt>],
    bg_color: Color,
}

impl<'txt, 'ln> DacttyloWidget<'txt, 'ln> {
    pub fn new(
        main: &'txt PlayerState,
        opponents: &'txt PlayerPool,
        lines: &'ln [StyledLine<'txt>],
    ) -> Self {
        Self {
            main,
            opponents,
            highlighted_content: lines,
            block: Default::default(),
            bg_color: Color::Reset,
        }
    }

    pub fn block(mut self, block: Block<'txt>) -> Self {
        self.block = block;
        self
    }

    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = color;
        self
    }

    fn get_main_style(&self) -> Option<(TextCoord, Style)> {
        if let Some(player_coords) = self.main.get_cursor_coord() {
            let style = Style::default();
            let neutral = style.bg(Color::White).fg(Color::Black);
            let wrong = style.bg(Color::Red).fg(Color::White);

            let style = match self.main.last_input() {
                Some(InputResult::Wrong(_)) => wrong,
                _ => neutral,
            };

            Some((player_coords, style))
        } else {
            None
        }
    }

    fn get_main_error_styles(&self) -> HashMap<TextCoord, Style> {
        let coords = self.main.get_error_coords();

        let style = Style::default();
        let yellow = style.bg(Color::Yellow).fg(Color::Black);

        coords.into_iter().map(|coord| (coord, yellow)).collect()
    }

    fn get_opponent_styles(&self) -> HashMap<TextCoord, Style> {
        let opponent_coords = self.opponents.get_cursor_coords();

        let style = Style::default();
        let grey = style.bg(Color::Rgb(20, 20, 20)).fg(Color::White);

        opponent_coords
            .into_iter()
            .map(|(coord, _)| (coord, grey))
            .collect()
    }

    fn apply_cursors(
        styles: HashMap<TextCoord, Style>,
        mut hl_lines: Vec<Vec<StyledGrapheme>>,
    ) -> Vec<Vec<StyledGrapheme>> {
        for (coord, style) in styles {
            hl_lines[coord.ln][coord.x].style = style;
        }

        hl_lines
    }
}

impl<'txt, 'ln> Widget for DacttyloWidget<'txt, 'ln> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut styles = self.get_opponent_styles();
        let error_styles = self.get_main_error_styles();
        styles.extend(error_styles);

        let main_style = self.get_main_style();
        if let Some((coord, style)) = &main_style {
            styles.insert(coord.clone(), *style);
        }

        let styled_lines =
            Self::apply_cursors(styles, self.highlighted_content.to_owned());

        let current_ln = main_style
            .map(|(coord, _)| coord.ln)
            .unwrap_or(styled_lines.len() - 1);

        TextView::from_styled_content(&styled_lines)
            .block(self.block)
            .anchor(Anchor::Center(current_ln))
            .bg_color(self.bg_color)
            .render(area, buf);
    }
}
