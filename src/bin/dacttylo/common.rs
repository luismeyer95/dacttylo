use dacttylo::{
    aggregate,
    app::{
        state::{PlayerPool, PlayerState},
        widget::DacttyloWidget,
    },
    events::{app_event, AppEvent, EventAggregator},
    game::game::Game,
    highlighting::{Highlighter, SyntectHighlighter},
    stats::SessionStats,
    utils::{
        syntect::syntect_load_defaults,
        types::{AsyncResult, StyledLine},
    },
    widgets::{figtext::FigTextWidget, wpm::WpmWidget},
};
use figlet_rs::FIGfont;
use once_cell::sync::OnceCell;
use std::{io::Stdout, time::Duration};
use syntect::highlighting::Theme;
use tokio::sync::mpsc::Sender;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::StyledGrapheme,
    widgets::{Block, Borders},
    Frame, Terminal,
};

pub enum SessionState {
    Ongoing,
    End(SessionEnd),
}

pub enum SessionEnd {
    Finished,
    Quit,
}

pub fn spawn_ticker(client: Sender<AppEvent>) {
    tokio::spawn(async move {
        loop {
            if client.send(AppEvent::WpmTick).await.is_err() {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

pub fn handle_wpm_tick(stats: &mut SessionStats, main: &PlayerState) {
    let recorder = &main.recorder;
    let record = recorder.record();
    let elapsed = recorder.elapsed();
    let wpm = record.wpm_at(Duration::from_secs(4), elapsed);

    stats.wpm_series.push((elapsed.as_secs_f64(), wpm));
    stats.average_wpm = record.average_wpm(recorder.elapsed());
    stats.top_wpm = f64::max(wpm, stats.top_wpm);
    stats.mistake_count = record.count_wrong();
    stats.precision = record.precision();
}

pub fn configure_event_stream() -> (Sender<AppEvent>, EventAggregator<AppEvent>)
{
    let (client, stream) = app_event::stream();
    spawn_ticker(client.clone());

    let term_io_stream = crossterm::event::EventStream::new();
    (client, aggregate!([stream, term_io_stream] as AppEvent))
}

pub fn get_theme(theme: &str) -> &'static Theme {
    let (_, ts) = syntect_load_defaults();
    &ts.themes[theme]
}

pub fn format_and_style<'t>(
    text: &'t str,
    file: &str,
    theme: &str,
) -> AsyncResult<Vec<Vec<StyledGrapheme<'t>>>> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();

    let hl = SyntectHighlighter::new()
        .from_file((file).into())?
        .theme(get_theme(theme))
        .build()?;

    Ok(hl.highlight(&lines))
}

pub fn render<O>(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &Game<O>,
    stats: &SessionStats,
    styled_lines: &[StyledLine],
) -> AsyncResult<()> {
    term.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(
                [Constraint::Length(7), Constraint::Percentage(60)].as_ref(),
            )
            .split(f.size());

        let wpm_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [Constraint::Percentage(80), Constraint::Percentage(20)]
                    .as_ref(),
            )
            .split(chunks[0]);
        render_dacttylo(f, wpm_chunks[0]);
        render_wpm(f, wpm_chunks[1], stats);
        render_text(
            f,
            chunks[1],
            &app.main,
            &app.opponents,
            styled_lines,
            &app.theme,
        );
    })?;

    Ok(())
}

pub fn load_wpm_font() -> &'static FIGfont {
    static FONT: OnceCell<FIGfont> = OnceCell::new();
    FONT.get_or_init(|| {
        let bytes = include_bytes!("figfonts/lcd.flf");
        let s = std::str::from_utf8(bytes).unwrap();
        FIGfont::from_content(s).unwrap()
    })
}

pub fn load_title_font() -> &'static FIGfont {
    static FONT: OnceCell<FIGfont> = OnceCell::new();
    FONT.get_or_init(|| {
        let bytes = include_bytes!("figfonts/slant.flf");
        let s = std::str::from_utf8(bytes).unwrap();
        FIGfont::from_content(s).unwrap()
    })
}

pub fn render_wpm(
    f: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
    stats: &SessionStats,
) {
    let wpm = stats.wpm_series.last().map_or(0.0, |(_, wpm)| *wpm);
    let widget = WpmWidget::new(wpm as u32, load_wpm_font());
    f.render_widget(widget, area);
}

pub fn render_text(
    f: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
    main: &PlayerState<'_>,
    opponents: &PlayerPool<'_>,
    styled_lines: &[StyledLine],
    theme: &str,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Reset).fg(Color::White));

    let bg = get_theme(theme).settings.background.unwrap();

    f.render_widget(
        DacttyloWidget::new(main, opponents, styled_lines)
            .block(block)
            .bg_color(Color::Rgb(bg.r, bg.g, bg.b)),
        area,
    );
}

pub fn render_dacttylo(f: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Reset).fg(Color::White));

    let font = load_title_font();
    let figtext = FigTextWidget::new("dacttylo", font)
        .align(Alignment::Center)
        .block(block);
    f.render_widget(figtext, area);
}
