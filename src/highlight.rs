use once_cell::sync::OnceCell;
use syntect::easy::HighlightLines;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet, util::LinesWithEndings};

/// Highlighter trait for applying global text styling before rendering a Typeview widget
pub trait Highlighter {
    fn highlight<'txt>(&self, text: &'txt str) -> Vec<(&'txt str, tui::style::Style)>;
}

/// A no-op default implementation
pub struct NoHighlight;
impl Highlighter for NoHighlight {
    fn highlight<'txt>(&self, text: &'txt str) -> Vec<(&'txt str, tui::style::Style)> {
        vec![(text, Default::default())]
    }
}

/// An implementation using the syntect highlighting engine
pub struct SyntectHighlight;
impl Highlighter for SyntectHighlight {
    fn highlight<'txt>(&self, text: &'txt str) -> Vec<(&'txt str, tui::style::Style)> {
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

        let mut tokenized_contents: Vec<(syntect::highlighting::Style, &str)> = vec![];
        for line in LinesWithEndings::from(&text) {
            let mut tokens: Vec<(syntect::highlighting::Style, &str)> =
                highlighter.highlight(&line, &syntax_set);
            tokenized_contents.extend(tokens);
        }

        tokenized_contents
            .into_iter()
            .map(|(style, token)| (token, Self::syntect_to_tui_style(style)))
            .collect()
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
