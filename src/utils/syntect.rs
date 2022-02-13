use once_cell::sync::OnceCell;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

pub fn syntect_load_defaults() -> (&'static SyntaxSet, &'static ThemeSet) {
    static SYNTAX_SET: OnceCell<SyntaxSet> = OnceCell::new();
    static THEME_SET: OnceCell<ThemeSet> = OnceCell::new();
    (
        SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines),
        THEME_SET.get_or_init(ThemeSet::load_defaults),
    )
}

pub fn syntect_to_tui_style(
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
