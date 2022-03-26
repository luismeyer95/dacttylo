use std::io::Stdout;

use crossterm::event::Event;
use dacttylo::{
    stats::SessionStats, utils::types::AsyncResult,
    widgets::figtext::FigTextWidget,
};
use figlet_rs::FIGfont;
use once_cell::sync::OnceCell;
use tokio_stream::StreamExt;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Text},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
    Frame, Terminal,
};

#[derive(Debug, Clone)]
pub struct SessionResult {
    pub stats: SessionStats,
    pub ranking: Option<Ranking>,
}
#[derive(Debug, Clone)]
pub struct Ranking {
    pub spot: usize,
    pub names: Vec<String>,
}

pub async fn display_session_report(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    session_result: SessionResult,
) -> AsyncResult<()> {
    render_report(term, &session_result).await?;

    let mut input_stream = crossterm::event::EventStream::new();
    while let Some(event) = input_stream.next().await {
        let event = event?;
        if let Event::Key(_) = event {
            break;
        }
        render_report(term, &session_result).await?;
    }

    Ok(())
}

async fn render_report(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    session_result: &SessionResult,
) -> AsyncResult<()> {
    term.draw(|f| {
        let block = Block::default().borders(Borders::ALL);
        let report_window = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(f.size())[0];
        f.render_widget(block.clone(), report_window);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(
                [
                    Constraint::Min(6),
                    Constraint::Percentage(40),
                    Constraint::Percentage(40),
                ]
                .as_ref(),
            )
            .split(report_window);

        render_header(f, chunks[0]);
        render_data(f, chunks[1], session_result);
        render_chart(f, chunks[2], &session_result.stats);
    })?;

    Ok(())
}

fn render_header<B: Backend>(frame: &mut Frame<B>, area: Rect) {
    let font = load_header_font();
    let figtext = FigTextWidget::new("REPORT", font)
        .color(Color::Red)
        .align(Alignment::Center);
    frame.render_widget(figtext, area);
}

fn render_data<B: Backend>(
    frame: &mut Frame<B>,
    area: Rect,
    session_result: &SessionResult,
) {
    match &session_result.ranking {
        None => render_stats(frame, area, &session_result.stats),
        Some(ranking) => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [Constraint::Percentage(50), Constraint::Percentage(50)]
                        .as_ref(),
                )
                .split(area);

            let stats_chunk = chunks[0];
            let ranking_chunk = chunks[1];

            render_stats(frame, stats_chunk, &session_result.stats);
            render_ranking(frame, ranking_chunk, ranking);
        }
    }
}

fn render_stats<B: Backend>(
    f: &mut Frame<B>,
    area: Rect,
    stats: &SessionStats,
) {
    let stats_fmt = format!("{}", stats);

    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Reset).fg(Color::White))
        .title(Span::styled(
            "Stats",
            Style::default().add_modifier(Modifier::BOLD),
        ));

    f.render_widget(block, area);

    let center = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(area)[0];

    let paragraph = Paragraph::new(Text::from(stats_fmt))
        .style(Style::default().bg(Color::Reset).fg(Color::White))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, center);
}

fn render_ranking<B: Backend>(
    frame: &mut Frame<B>,
    area: Rect,
    ranking: &Ranking,
) {
    let podium = ["🥇", "🥈", "🥉"];
    let text = ranking
        .names
        .iter()
        .enumerate()
        .map(|(i, name)| match podium.get(i) {
            Some(&medal) => format!("{medal} {name}"),
            None => format!("💩 {name}"),
        })
        .collect::<Vec<_>>()
        .join("\r\n");

    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Reset).fg(Color::White))
        .title(Span::styled(
            "Ranking",
            Style::default().add_modifier(Modifier::BOLD),
        ));

    frame.render_widget(block, area);

    let center = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(area)[0];

    let paragraph = Paragraph::new(Text::from(text))
        .style(Style::default().bg(Color::Reset).fg(Color::White))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, center);
}

fn render_chart(
    f: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
    stats: &SessionStats,
) {
    let data = stats.wpm_series.as_slice();

    let last = data.last().map_or(0.0, |(secs, _)| *secs);
    let x_bounds = [0.0, last];

    let datasets = vec![Dataset::default()
        .name("WPM")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Yellow))
        .data(data)];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(Span::styled(
                    "WPM Over Time",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Seconds")
                .style(Style::default().fg(Color::Gray))
                .bounds(x_bounds),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .labels(vec![
                    Span::styled(
                        "0",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "100",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ])
                .bounds([0.0, 150.0]),
        );
    f.render_widget(chart, area);
}

fn load_header_font() -> &'static FIGfont {
    static FONT: OnceCell<FIGfont> = OnceCell::new();
    FONT.get_or_init(|| {
        let bytes = include_bytes!("figfonts/slant.flf");
        let s = std::str::from_utf8(bytes).unwrap();
        FIGfont::from_content(s).unwrap()
    })
}
