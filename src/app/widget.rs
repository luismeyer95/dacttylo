use crate::highlighting::{Highlighter, NoOpHighlighter};
use crate::text_view::RenderMetadata;
use crate::utils::helpers;
use crate::{
    highlighting::SyntectHighlighter,
    text_coord::TextCoord,
    text_view::{Anchor, TextView},
};
use std::collections::HashMap;
use tui::text::StyledGrapheme;
use tui::widgets::StatefulWidget;
use tui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};
use unicode_segmentation::UnicodeSegmentation;

use crate::app::InputResult;

use super::state::DacttyloGameState;

pub struct DacttyloWidget<'txt, 'hl> {
    game_state: &'txt DacttyloGameState<'txt>,
    highlighter: &'hl dyn Highlighter,
}

impl<'txt, 'hl> DacttyloWidget<'txt, 'hl> {
    pub fn new(game_state: &'txt DacttyloGameState) -> Self {
        Self {
            game_state,
            highlighter: &NoOpHighlighter,
        }
    }

    pub fn highlighter(mut self, highlighter: &'hl dyn Highlighter) -> Self {
        self.highlighter = highlighter;
        self
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

type StyledLineIterator<'a> = Box<dyn Iterator<Item = StyledGrapheme<'a>> + 'a>;

impl<'txt, 'hl> StatefulWidget for DacttyloWidget<'txt, 'hl> {
    type State = Option<RenderMetadata>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let text_lines: Vec<&str> =
            self.game_state.text().split_inclusive('\n').collect();

        let styled_lines: Vec<Box<dyn Iterator<Item = StyledGrapheme<'txt>>>> =
            self.highlighter
                .highlight(text_lines.as_ref())
                .into_iter()
                .map(|tokens| {
                    Box::new(tokens.into_iter().flat_map(|(token, style)| {
                        token
                            .graphemes(true)
                            .map(move |g| StyledGrapheme { symbol: g, style })
                    }))
                        as Box<dyn Iterator<Item = StyledGrapheme<'txt>>>
                })
                .collect();

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
            .styled_content(styled_lines.into_iter())
            .anchor(Anchor::Center(main_coord.0));
        view.render(area, buf, state);
    }
}
