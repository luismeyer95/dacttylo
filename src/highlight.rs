use itertools::Itertools;
use once_cell::sync::OnceCell;
use syntect::easy::HighlightLines;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet, util::LinesWithEndings};

/// Highlighter trait for applying global text styling before rendering a Typeview widget
pub trait Highlighter {
    fn highlight<'txt>(&self, lines: &[&'txt str]) -> Vec<Vec<(&'txt str, tui::style::Color)>>;
}

/// A no-op default implementation
pub struct NoHighlight;
impl Highlighter for NoHighlight {
    fn highlight<'txt>(&self, lines: &[&'txt str]) -> Vec<Vec<(&'txt str, tui::style::Color)>> {
        lines
            .into_iter()
            .map(|&s| vec![(s, tui::style::Color::White)])
            .collect()
    }
}

/// An implementation using the syntect highlighting engine
pub struct SyntectHighlight;
impl Highlighter for SyntectHighlight {
    fn highlight<'txt>(&self, lines: &[&'txt str]) -> Vec<Vec<(&'txt str, tui::style::Color)>> {
        let (syntax_set, theme_set) = Self::load_defaults();
        let syntax = syntax_set
            .find_syntax_by_extension("rs")
            .expect("syntax extension not found");

        let themes = [
            "Solarized (dark)",
            "Solarized (light)",
            "base16-ocean.dark",
            "base16-eighties.dark",
            "base16-mocha.dark",
            "base16-ocean.light",
            "InspiredGitHub",
        ];

        let mut highlighter = HighlightLines::new(syntax, &theme_set.themes[themes[0]]);
        let mut tokenized_lines: Vec<Vec<(&str, tui::style::Color)>> = vec![];

        for line in lines {
            let tokens = highlighter.highlight(&line, &syntax_set);
            tokenized_lines.push(
                tokens
                    .into_iter()
                    .map(|(style, token)| {
                        (
                            token,
                            tui::style::Color::Rgb(
                                style.foreground.r,
                                style.foreground.g,
                                style.foreground.b,
                            ),
                        )
                    })
                    .collect(),
            );
        }

        tokenized_lines
    }
}

impl SyntectHighlight {
    fn load_defaults() -> (&'static SyntaxSet, &'static ThemeSet) {
        static SYNTAX_SET: OnceCell<SyntaxSet> = OnceCell::new();
        static THEME_SET: OnceCell<ThemeSet> = OnceCell::new();
        (
            SYNTAX_SET.get_or_init(|| SyntaxSet::load_defaults_newlines()),
            THEME_SET.get_or_init(|| ThemeSet::load_defaults()),
        )
    }

    fn syntect_to_tui_style(syntect_style: syntect::highlighting::Style) -> tui::style::Style {
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
}
