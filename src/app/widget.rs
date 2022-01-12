use crate::highlight::Highlighter;
use crate::utils::helpers;
use crate::{
    highlight::SyntectHighlight,
    text_coord::TextCoord,
    text_view::{Anchor, TextView},
};
use std::{collections::HashMap, ops::Range};
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    widgets::{StatefulWidget, Widget},
};

use crate::app::InputResult;

use super::state::DacttyloGameState;

type StyledLine<'a> = Vec<(&'a str, tui::style::Style)>;

pub struct RenderMetadata {
    pub buffer_height: u16,
    pub lines_rendered: Range<usize>,
}

pub struct DacttyloWidget<'txt> {
    game_state: &'txt DacttyloGameState<'txt>,
}

impl<'txt> DacttyloWidget<'txt> {
    pub fn new(game_state: &'txt DacttyloGameState) -> Self {
        Self { game_state }
    }

    pub fn get_cursor_styles(&self, lines: &[&'txt str]) -> HashMap<TextCoord, tui::style::Style> {
        let state = self.game_state;

        let mut index_map = state
            .players()
            .iter()
            .map(|(_, pstate)| (pstate.cursor(), pstate.last_input()))
            .collect::<HashMap<_, _>>();

        // Making sure the main player cursor takes precedence over the others
        let main_player = state.main_player().unwrap();
        index_map.insert(main_player.cursor(), main_player.last_input());

        let mut style_map: HashMap<TextCoord, tui::style::Style> = HashMap::new();

        let style = tui::style::Style::default();
        let neutral = style.bg(Color::White).fg(Color::Black);
        let wrong = style.bg(Color::Red).fg(Color::White);

        // TODO: this works for now but is definitely not optimal
        let mut count = 0;
        for (ln_index, &ln) in lines.iter().enumerate() {
            for (ch_index, _) in ln.chars().enumerate() {
                if let Some(input_result) = index_map.get(&count) {
                    let cursor_style = match input_result {
                        Some(InputResult::Wrong(_)) => wrong,
                        _ => neutral,
                    };
                    style_map.insert(TextCoord::new(ln_index, ch_index), cursor_style);
                }
                count += 1;
            }
        }

        style_map
    }
}

impl<'txt> Widget for DacttyloWidget<'txt> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut hl = SyntectHighlight::new();

        let text_lines: Vec<&str> = self.game_state.text().split_inclusive('\n').collect();

        let styled_lines: Vec<StyledLine<'txt>> = text_lines
            .iter()
            .map(|s| {
                hl.highlight_line(s)
                    .into_iter()
                    .map(|(tkn, color)| (tkn, tui::style::Style::default().fg(color)))
                    .collect()
            })
            .collect();

        // let eggshell = Color::Rgb(255, 239, 214);
        let darkblue = Color::Rgb(0, 27, 46);
        let main_player = self.game_state.main_player().unwrap();
        let main_coord =
            helpers::text_to_line_index(vec![main_player.cursor()], &text_lines).unwrap()[0];

        let view = TextView::new()
            .sparse_styling(self.get_cursor_styles(&text_lines))
            .styled_content(styled_lines)
            .anchor(Anchor::Center(main_coord.0))
            .bg_color(darkblue);
        view.render(area, buf);
    }
}
