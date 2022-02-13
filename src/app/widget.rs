use crate::utils::helpers;
use crate::{
    text_coord::TextCoord,
    text_view::{Anchor, TextView},
};
use std::collections::HashMap;
use std::iter;
use tui::style::Style;
use tui::text::StyledGrapheme;
use tui::widgets::Block;
use tui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};
use unicode_segmentation::UnicodeSegmentation;

use crate::app::InputResult;

use super::state::{PlayerPool, PlayerState};

pub struct DacttyloWidget<'txt> {
    block: Block<'txt>,

    main: &'txt PlayerState<'txt>,
    opponents: &'txt PlayerPool<'txt>,

    highlighted_content: Option<Vec<Vec<StyledGrapheme<'txt>>>>,
    bg_color: Color,
}

impl<'txt> DacttyloWidget<'txt> {
    pub fn new(main: &'txt PlayerState, opponents: &'txt PlayerPool) -> Self {
        Self {
            main,
            opponents,
            highlighted_content: None,
            block: Default::default(),
            bg_color: Color::Reset,
        }
    }

    pub fn highlighted_content(
        mut self,
        highlighted_content: Vec<Vec<StyledGrapheme<'txt>>>,
    ) -> Self {
        self.highlighted_content = Some(highlighted_content);
        self
    }

    pub fn block(mut self, block: Block<'txt>) -> Self {
        self.block = block;
        self
    }

    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = color;
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

impl<'txt> Widget for DacttyloWidget<'txt> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let main_coord = self.main.get_cursor_coord();

        let mut styles = self.get_opponent_styles();
        let main_style = self.get_main_style();
        styles.insert(main_style.0, main_style.1);

        let hl_lines = self.highlighted_content.unwrap();
        let styled_lines = Self::apply_cursors(styles, hl_lines);

        TextView::new()
            .block(self.block)
            .anchor(Anchor::Center(main_coord.ln))
            .styled_content(styled_lines)
            .bg_color(self.bg_color)
            .render(area, buf);
    }
}
