use super::highlighter::Highlighter;
use once_cell::sync::OnceCell;
use std::error::Error;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

/// An implementation using the syntect highlighting engine
pub struct SyntectHighlighter {
    syntax_set: &'static SyntaxSet,
    highlighter: HighlightLines<'static>,
}

#[allow(clippy::new_ret_no_self)]
impl SyntectHighlighter {
    pub fn new() -> SyntectHighlighterBuilder {
        Default::default()
    }
}

impl Highlighter for SyntectHighlighter {
    fn highlight<'txt>(
        &mut self,
        lines: &[&'txt str],
    ) -> Vec<Vec<(&'txt str, tui::style::Style)>> {
        lines.iter().map(|ln| self.highlight_line(ln)).collect()
    }

    fn highlight_line<'txt>(
        &mut self,
        line: &'txt str,
    ) -> Vec<(&'txt str, tui::style::Style)> {
        let tokens = self.highlighter.highlight(line, self.syntax_set);
        tokens
            .into_iter()
            .map(|(style, token)| (token, syntect_to_tui_style(style)))
            .collect()
    }
}

fn load_defaults() -> (&'static SyntaxSet, &'static ThemeSet) {
    static SYNTAX_SET: OnceCell<SyntaxSet> = OnceCell::new();
    static THEME_SET: OnceCell<ThemeSet> = OnceCell::new();
    (
        SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines),
        THEME_SET.get_or_init(ThemeSet::load_defaults),
    )
}

fn syntect_to_tui_style(
    syntect_style: syntect::highlighting::Style,
) -> tui::style::Style {
    use syntect::highlighting::FontStyle;
    use tui::style::Modifier;
    let mut style = tui::style::Style::default()
        .fg(tui::style::Color::Rgb(
            syntect_style.foreground.r,
            syntect_style.foreground.g,
            syntect_style.foreground.b,
        ))
        .bg(tui::style::Color::Rgb(
            syntect_style.background.r,
            syntect_style.background.g,
            syntect_style.background.b,
        ));
    if syntect_style.font_style.contains(FontStyle::BOLD) {
        style = style.add_modifier(Modifier::BOLD)
    }
    if syntect_style.font_style.contains(FontStyle::UNDERLINE) {
        style = style.add_modifier(Modifier::UNDERLINED)
    }
    if syntect_style.font_style.contains(FontStyle::ITALIC) {
        style = style.add_modifier(Modifier::ITALIC)
    }

    style
}

#[derive(Debug, Clone)]
pub struct SyntectHighlighterBuilder {
    extension: String,
    theme: String,
}

impl Default for SyntectHighlighterBuilder {
    fn default() -> Self {
        Self {
            extension: Default::default(),
            theme: Self::DEFAULT_THEMES[1].into(),
        }
    }
}

impl SyntectHighlighterBuilder {
    const DEFAULT_THEMES: [&'static str; 7] = [
        "Solarized (dark)",
        "Solarized (light)",
        "base16-ocean.dark",
        "base16-eighties.dark",
        "base16-mocha.dark",
        "base16-ocean.light",
        "InspiredGitHub",
    ];

    pub fn extension(mut self, ext: impl Into<String>) -> Self {
        self.extension = ext.into();
        self
    }

    pub fn theme(mut self, theme: impl Into<String>) -> Self {
        self.theme = theme.into();
        self
    }

    pub fn build(self) -> Result<SyntectHighlighter, Box<dyn Error>> {
        let (syntax_set, theme_set) = load_defaults();
        let syntax = syntax_set
            .find_syntax_by_extension(&self.extension)
            .ok_or("syntax extension not found")?;

        let highlighter =
            HighlightLines::new(syntax, &theme_set.themes[&self.theme]);

        Ok(SyntectHighlighter {
            syntax_set,
            highlighter,
        })
    }
}
