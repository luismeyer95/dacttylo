use std::cmp::min;

use figlet_rs::{FIGfont, FIGure};
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Widget},
};

pub struct WpmWidget<'f> {
    wpm: u32,
    font: &'f FIGfont,
}

impl<'f> WpmWidget<'f> {
    pub fn new(wpm: u32, font: &'f FIGfont) -> Self {
        Self { wpm, font }
    }
}

impl<'f> Widget for WpmWidget<'f> {
    fn render(self, mut area: Rect, buf: &mut Buffer) {
        render_block(&mut area, buf);

        let figure = self.font.convert(&self.wpm.to_string()).unwrap();
        let rows = figure_to_rows(figure);

        let (offset_x, offset_y) = (
            compute_offset_x(area.width, &rows),
            compute_offset_y(area.height, &rows),
        );

        let style = Style::default().fg(speed_color(self.wpm));
        let max_height = min(rows.len() as u16, area.height);

        for (i, y) in (area.top() + offset_y
            ..area.top() + offset_y + max_height)
            .enumerate()
        {
            buf.set_string(area.left() + offset_x, y, &rows[i], style);
        }
    }
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

fn render_block(area: &mut Rect, buf: &mut Buffer) {
    let block = Block::default()
        .title(Span::styled(
            "WPM",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL);
    // save the inner_area because render consumes the block
    let inner_area = block.inner(*area);
    block.render(*area, buf);

    *area = inner_area;
}

fn figure_to_rows(figure: FIGure) -> Vec<String> {
    let mut rows: Vec<String> = vec![];

    for y in 0..figure.height {
        let mut row = String::new();
        for ch in &figure.characters {
            row.push_str(&ch.characters[y as usize]);
        }
        rows.push(row);
    }

    rows
}

fn compute_offset_x(total_width: u16, rows: &[String]) -> u16 {
    let fig_width = rows[0].chars().count() as u16;

    (total_width.saturating_sub(fig_width)) / 2
}

fn compute_offset_y(total_height: u16, rows: &[String]) -> u16 {
    let fig_height = rows.len() as u16;

    (total_height.saturating_sub(fig_height)) / 2
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
