use std::cmp::min;

use figlet_rs::{FIGfont, FIGure};
use tui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Widget},
};

pub struct FigTextWidget<'f, 'b> {
    font: &'f FIGfont,
    s: String,
    color: Option<Color>,
    block: Option<Block<'b>>,
    alignment: Option<Alignment>,
}

impl<'f, 'b> FigTextWidget<'f, 'b> {
    pub fn new(s: &str, font: &'f FIGfont) -> Self {
        Self {
            s: s.into(),
            font,
            color: None,
            block: None,
            alignment: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color.into();
        self
    }

    pub fn block(mut self, block: Block<'b>) -> Self {
        self.block = block.into();
        self
    }

    pub fn align(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment.into();
        self
    }
}

impl<'f, 'b> Widget for FigTextWidget<'f, 'b> {
    fn render(mut self, mut area: Rect, buf: &mut Buffer) {
        if let Some(block) = self.block.take() {
            render_block(block, &mut area, buf);
        }

        let figure = self.font.convert(&self.s).unwrap();
        let rows = figure_to_rows(figure);

        let (offset_x, offset_y) = (
            compute_offset_x(area.width, &rows, self.alignment),
            compute_offset_y(area.height, &rows),
        );

        let style = Style::default().fg(self.color.unwrap_or(Color::White));
        let max_height = min(rows.len() as u16, area.height);

        for (i, y) in (area.top() + offset_y
            ..area.top() + offset_y + max_height)
            .enumerate()
        {
            buf.set_stringn(
                area.left() + offset_x,
                y,
                &rows[i],
                area.width as usize,
                style,
            );
        }
    }
}

fn render_block(block: Block, area: &mut Rect, buf: &mut Buffer) {
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

fn compute_offset_x(
    total_width: u16,
    rows: &[String],
    alignment: Option<Alignment>,
) -> u16 {
    let alignment = alignment.unwrap_or(Alignment::Left);
    let fig_width = rows[0].chars().count() as u16;

    match alignment {
        Alignment::Left => 0,
        Alignment::Center => (total_width.saturating_sub(fig_width)) / 2,
        Alignment::Right => (total_width.saturating_sub(fig_width)),
    }
}

fn compute_offset_y(total_height: u16, rows: &[String]) -> u16 {
    let fig_height = rows.len() as u16;
    (total_height.saturating_sub(fig_height)) / 2
}
