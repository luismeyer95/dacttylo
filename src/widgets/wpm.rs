use figlet_rs::FIGfont;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Widget},
};

pub struct WpmWidget(pub u32);

impl WpmWidget {
    fn render_block(block: Block, area: &mut Rect, buf: &mut Buffer) {
        // save the inner_area because render consumes the block
        let inner_area = block.inner(*area);
        block.render(*area, buf);

        *area = inner_area;
    }

    fn speed_color(wpm: u32) -> Color {
        match wpm {
            0..=49 => Color::LightGreen,
            50..=69 => Color::LightYellow,
            70..=89 => Color::LightRed,
            90..=109 => Color::LightMagenta,
            _ => Color::LightCyan,
        }
    }
}

impl Widget for WpmWidget {
    fn render(self, mut area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(Span::styled(
                "WPM",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL);

        Self::render_block(block, &mut area, buf);

        let font = FIGfont::from_file("figfonts/lcd.flf").unwrap();
        let figure = font.convert(&format!("{}", self.0)).unwrap();

        let mut rows: Vec<String> = vec![];

        for y in 0..figure.height {
            let mut row = String::new();
            for ch in &figure.characters {
                row.push_str(&ch.characters[y as usize]);
            }
            rows.push(row);
        }

        let fig_width = rows[0].chars().count() as u16;
        let fig_height = figure.height as u16;

        let offset_x = (area.width.saturating_sub(fig_width)) / 2;
        let offset_y = (area.height.saturating_sub(fig_height)) / 2;

        let style = Style::default().fg(Self::speed_color(self.0));

        for (i, y) in (area.top() + offset_y
            ..area.top() + offset_y + fig_height)
            .enumerate()
        {
            buf.set_string(area.left() + offset_x, y, &rows[i], style);
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;

    fn gen_wpm() -> String {
        let wpm = rand::thread_rng().gen_range(0..160);
        format!("{}", wpm)
    }

    #[test]
    fn basic() {
        let small_font = FIGfont::from_file("figfonts/lcd.flf").unwrap();
        let figure = small_font.convert("127");
        assert!(figure.is_some());
    }

    #[test]
    fn max_width() {
        let small_font = FIGfont::from_file("figfonts/lcd.flf").unwrap();
        let mut wh_pairs: Vec<(u32, u32)> = vec![];

        let figure = small_font.convert("0123456.789").unwrap();
        for c in &figure.characters {
            wh_pairs.push((c.width, c.height));
        }

        println!("{:?}", wh_pairs);
        println!("{}", figure);

        // assert!(figure.is_some());
    }
}
