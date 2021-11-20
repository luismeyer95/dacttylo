#[derive(Debug, Clone, Default)]
pub struct Style(tui::style::Style);

type TuiStyle = tui::style::Style;
type SyntectStyle = syntect::highlighting::Style;
type SyntectMod = syntect::highlighting::FontStyle;
type TuiMod = tui::style::Modifier;
type TuiColor = tui::style::Color;

impl From<SyntectStyle> for Style {
    fn from(syntect_style: SyntectStyle) -> Self {
        let mut style = TuiStyle::default()
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
        if syntect_style.font_style.contains(SyntectMod::BOLD) {
            style = style.add_modifier(TuiMod::BOLD)
        }
        if syntect_style.font_style.contains(SyntectMod::UNDERLINE) {
            style = style.add_modifier(TuiMod::UNDERLINED)
        }
        if syntect_style.font_style.contains(SyntectMod::ITALIC) {
            style = style.add_modifier(TuiMod::ITALIC)
        }

        Self(style)
    }
}

impl From<TuiStyle> for Style {
    fn from(tui_style: TuiStyle) -> Self {
        Self(tui_style)
    }
}

impl Into<TuiStyle> for Style {
    fn into(self) -> TuiStyle {
        self.0
    }
}
