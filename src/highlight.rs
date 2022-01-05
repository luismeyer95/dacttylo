use once_cell::sync::OnceCell;
use syntect::easy::HighlightLines;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

/// Highlighter trait for applying global text styling before rendering a Typeview widget
pub trait Highlighter {
    fn highlight<'txt>(&mut self, lines: &[&'txt str]) -> Vec<Vec<(&'txt str, tui::style::Color)>>;
    fn highlight_line<'txt>(&mut self, line: &'txt str) -> Vec<(&'txt str, tui::style::Color)>;
}

/// A no-op default implementation
pub struct NoHighlight;
impl Highlighter for NoHighlight {
    fn highlight<'txt>(&mut self, lines: &[&'txt str]) -> Vec<Vec<(&'txt str, tui::style::Color)>> {
        lines.iter().map(|&s| self.highlight_line(s)).collect()
    }

    fn highlight_line<'txt>(&mut self, line: &'txt str) -> Vec<(&'txt str, tui::style::Color)> {
        vec![(line, tui::style::Color::White)]
    }
}

/// An implementation using the syntect highlighting engine
pub struct SyntectHighlight {
    syntax_set: &'static SyntaxSet,
    highlighter: HighlightLines<'static>,
}

impl SyntectHighlight {
    pub fn new() -> SyntectHighlight {
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

        let highlighter = HighlightLines::new(syntax, &theme_set.themes[themes[2]]);

        SyntectHighlight {
            syntax_set,
            highlighter,
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
impl Highlighter for SyntectHighlight {
    fn highlight<'txt>(&mut self, lines: &[&'txt str]) -> Vec<Vec<(&'txt str, tui::style::Color)>> {
        let mut tokenized_lines: Vec<Vec<(&str, tui::style::Color)>> =
            Vec::<_>::with_capacity(lines.len());

        for line in lines {
            tokenized_lines.push(self.highlight_line(line));
        }

        tokenized_lines
    }

    fn highlight_line<'txt>(&mut self, line: &'txt str) -> Vec<(&'txt str, tui::style::Color)> {
        let tokens = self.highlighter.highlight(line, self.syntax_set);
        tokens
            .into_iter()
            .map(|(style, token)| {
                (
                    token,
                    // TODO: forgot about modifiers...
                    tui::style::Color::Rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    ),
                )
            })
            .collect()
    }
}
