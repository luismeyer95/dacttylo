use super::highlighter::Highlighter;
use crate::utils::syntect::{syntect_load_defaults, syntect_to_tui_style};
use crate::utils::types::AsyncResult;
use std::cell::RefCell;
use syntect::easy::HighlightLines;
use syntect::highlighting::Theme;
use syntect::parsing::{SyntaxReference, SyntaxSet};
use tui::text::StyledGrapheme;
use unicode_segmentation::UnicodeSegmentation;

/// An implementation using the syntect highlighting engine
pub struct SyntectHighlighter<'s> {
    syntax_set: &'s SyntaxSet,
    highlighter: RefCell<HighlightLines<'s>>,
}

#[allow(clippy::new_ret_no_self)]
impl<'s> SyntectHighlighter<'s> {
    pub fn new() -> SyntectHighlighterBuilder<'s> {
        Default::default()
    }
}

impl<'s> Highlighter for SyntectHighlighter<'s> {
    fn highlight<'txt>(
        &self,
        lines: &[&'txt str],
    ) -> Vec<Vec<StyledGrapheme<'txt>>> {
        lines.iter().map(|ln| self.highlight_line(ln)).collect()
    }

    fn highlight_line<'txt>(
        &self,
        line: &'txt str,
    ) -> Vec<StyledGrapheme<'txt>> {
        let tokens = self
            .highlighter
            .borrow_mut()
            .highlight(line, self.syntax_set);

        let tui_tokens = tokens
            .into_iter()
            .map(|(style, token)| (token, syntect_to_tui_style(style)))
            .collect::<Vec<_>>();

        tokens_to_graphemes(&tui_tokens)
    }
}

fn tokens_to_graphemes<'tkn>(
    tokens: &[(&'tkn str, tui::style::Style)],
) -> Vec<StyledGrapheme<'tkn>> {
    tokens
        .iter()
        .flat_map(|(token, style)| {
            token.graphemes(true).map(|g| StyledGrapheme {
                symbol: g,
                style: *style,
            })
        })
        .collect::<Vec<StyledGrapheme<'tkn>>>()
}

#[derive(Debug, Clone)]
pub struct SyntectHighlighterBuilder<'a> {
    syntax: &'a SyntaxReference,
    theme: &'a Theme,
}

impl<'a> Default for SyntectHighlighterBuilder<'a> {
    fn default() -> Self {
        let (syntax_set, theme_set) = syntect_load_defaults();

        Self {
            syntax: syntax_set.find_syntax_plain_text(),
            theme: &theme_set.themes[Self::DEFAULT_THEMES[0]],
        }
    }
}

impl<'a> SyntectHighlighterBuilder<'a> {
    const DEFAULT_THEMES: [&'static str; 7] = [
        "Solarized (dark)",
        "Solarized (light)",
        "base16-ocean.dark",
        "base16-eighties.dark",
        "base16-mocha.dark",
        "base16-ocean.light",
        "InspiredGitHub",
    ];

    pub fn from_file<T>(mut self, file: Option<T>) -> AsyncResult<Self>
    where
        T: AsRef<str>,
    {
        if let Some(file) = file {
            let (syntax_set, _) = syntect_load_defaults();
            self.syntax = syntax_set
                .find_syntax_for_file(file.as_ref())
                .map_err(|_| "error reading file")?
                .ok_or("failed to find syntax")?;
        }

        Ok(self)
    }

    pub fn from_text<T>(mut self, text: T) -> AsyncResult<Self>
    where
        T: AsRef<str>,
    {
        let (syntax_set, _) = syntect_load_defaults();
        self.syntax = syntax_set
            .find_syntax_by_first_line(text.as_ref())
            .ok_or("failed to find syntax")?;

        Ok(self)
    }

    pub fn from_syntax<T>(mut self, name: T) -> AsyncResult<Self>
    where
        T: AsRef<str>,
    {
        let (syntax_set, _) = syntect_load_defaults();
        self.syntax = syntax_set
            .find_syntax_by_name(name.as_ref())
            .ok_or("failed to find syntax")?;

        Ok(self)
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn build(self) -> AsyncResult<SyntectHighlighter<'a>> {
        let (syntax_set, _) = syntect_load_defaults();

        let highlighter = HighlightLines::new(self.syntax, self.theme);

        Ok(SyntectHighlighter {
            syntax_set,
            highlighter: RefCell::new(highlighter),
        })
    }
}
