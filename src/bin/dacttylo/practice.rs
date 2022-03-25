use crate::AsyncResult;
use crossterm::event::{Event, KeyCode, KeyEvent};
use dacttylo::{
    aggregate,
    app::{
        state::{PlayerPool, PlayerState},
        widget::DacttyloWidget,
        InputResult, Progress,
    },
    cli::PracticeOptions,
    events::{app_event, AppEvent, EventAggregator},
    ghost::Ghost,
    highlighting::{Highlighter, SyntectHighlighter},
    record::{manager::RecordManager, recorder::InputResultRecorder},
    stats::SessionStats,
    utils::{
        syntect::syntect_load_defaults,
        tui::{enter_tui_mode, leave_tui_mode},
        types::StyledLine,
    },
    widgets::wpm::WpmWidget,
};
use figlet_rs::FIGfont;
use once_cell::sync::OnceCell;
use std::{io::Stdout, time::Duration};
use syntect::highlighting::Theme;
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, StyledGrapheme},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Frame, Terminal,
};

const THEME: &str = "Solarized (dark)";
pub struct SessionResult;

enum SessionState {
    Ongoing,
    End(SessionEnd),
}

enum SessionEnd {
    Finished(SessionResult),
    Quit,
}

// test commit

pub async fn init_practice_session(
    practice_opts: PracticeOptions,
) -> AsyncResult<()> {
    let result = run_practice_session(practice_opts).await;

    result
}

async fn run_practice_session(
    practice_opts: PracticeOptions,
) -> AsyncResult<()> {
    let text = std::fs::read_to_string(&practice_opts.file)?;
    let (main, opponents) = initialize_players(&text);

    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let styled_lines = apply_highlighting(&lines, &practice_opts.file)?;

    let (client, events) = configure_event_stream();
    // let mut ghost = initialize_ghost(&text, client.clone())?;
    spawn_ticker(client.clone());

    // ghost.start().await?;

    let mut term = enter_tui_mode(std::io::stdout())?;
    let session_result =
        handle_events(&mut term, main, opponents, events, client, styled_lines)
            .await;
    leave_tui_mode(term)?;

    // display session results
    Ok(())
}

pub fn spawn_ticker(client: Sender<AppEvent>) {
    tokio::spawn(async move {
        loop {
            client.send(AppEvent::WpmTick).await.unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

pub fn initialize_ghost(
    text: &str,
    client: Sender<AppEvent>,
) -> AsyncResult<Ghost> {
    let input_record =
        RecordManager::mount_dir("records")?.load_from_contents(text)?;
    Ok(Ghost::new(input_record, client))
}

pub fn initialize_players(text: &'_ str) -> (PlayerState<'_>, PlayerPool<'_>) {
    let main = PlayerState::new(text);
    let opponents = PlayerPool::new(text).with_players(&["ghost"]);
    (main, opponents)
}

pub fn configure_event_stream() -> (Sender<AppEvent>, EventAggregator<AppEvent>)
{
    let (client, stream) = app_event::stream();
    let term_io_stream = crossterm::event::EventStream::new();
    (client, aggregate!([stream, term_io_stream] as AppEvent))
}

pub fn get_theme(theme: &str) -> &'static Theme {
    let (_, ts) = syntect_load_defaults();
    &ts.themes[theme]
}

pub fn apply_highlighting<'t>(
    lines: &[&'t str],
    file: &str,
) -> AsyncResult<Vec<Vec<StyledGrapheme<'t>>>> {
    let hl = SyntectHighlighter::new()
        .from_file(file.into())?
        .theme(get_theme(THEME))
        .build()?;

    Ok(hl.highlight(lines))
}

async fn handle_events(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    mut main: PlayerState<'_>,
    mut opponents: PlayerPool<'_>,
    mut events: EventAggregator<AppEvent>,
    client: Sender<AppEvent>,
    styled_lines: Vec<Vec<StyledGrapheme<'_>>>,
) -> AsyncResult<SessionEnd> {
    let mut recorder = InputResultRecorder::new();
    let mut stats = SessionStats::default();

    while let Some(event) = events.next().await {
        let session_state = match event {
            AppEvent::Term(e) => handle_term(e?, &mut main, &mut recorder),
            AppEvent::GhostInput(c) => handle_ghost_input(c, &mut opponents),
            AppEvent::WpmTick => handle_wpm_tick(&mut stats, &recorder),
            _ => SessionState::Ongoing,
        };

        if let SessionState::End(end) = session_state {
            return Ok(end);
        }

        render(term, &main, &opponents, &stats, &styled_lines)?;
    }

    unreachable!();
}

fn handle_term(
    term_event: crossterm::event::Event,
    main: &mut PlayerState<'_>,
    recorder: &mut InputResultRecorder,
) -> SessionState {
    if let Event::Key(event) = term_event {
        let KeyEvent { code, .. } = event;
        let c = match code {
            KeyCode::Esc => return SessionState::End(SessionEnd::Quit),
            KeyCode::Char(c) => Some(c),
            KeyCode::Enter => Some('\n'),
            KeyCode::Tab => Some('\t'),
            _ => None,
        };

        if let Some(c) = c {
            let input_result = main.process_input(c).unwrap();
            recorder.push(input_result);

            if let InputResult::Correct(Progress::Finished) = input_result {
                // let manager = RecordManager::mount_dir("records").unwrap();
                // manager.save(main.text(), recorder.record()).unwrap();
                return SessionState::End(SessionEnd::Finished(SessionResult));
            }
        }
    }

    SessionState::Ongoing
}

fn handle_wpm_tick(
    stats: &mut SessionStats,
    recorder: &InputResultRecorder,
) -> SessionState {
    let record = recorder.record();
    let elapsed = recorder.elapsed();
    let wpm = record.wpm_at(Duration::from_secs(4), elapsed);

    stats.wpm_series.push((elapsed.as_secs_f64(), wpm));
    stats.average_wpm = record.average_wpm(recorder.elapsed());
    stats.top_wpm = f64::max(wpm, stats.top_wpm);
    stats.mistake_count = record.count_wrong();
    stats.precision = record.precision();

    SessionState::Ongoing
}

fn handle_ghost_input(
    input: InputResult,
    opponents: &mut PlayerPool,
) -> SessionState {
    match input {
        InputResult::Correct(Progress::Finished) => {
            SessionState::End(SessionEnd::Finished(SessionResult))
        }
        InputResult::Correct(Progress::Ongoing) => {
            opponents.advance_player("ghost").unwrap();
            SessionState::Ongoing
        }
        _ => SessionState::Ongoing,
    }
}

fn render(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    main: &PlayerState<'_>,
    opponents: &PlayerPool<'_>,
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
        render_chart(f, wpm_chunks[0], stats);
        render_wpm(f, wpm_chunks[1], stats);

        render_text(f, chunks[1], main, opponents, styled_lines);
    })?;

    Ok(())
}

pub fn load_font() -> &'static FIGfont {
    static FONT: OnceCell<FIGfont> = OnceCell::new();
    FONT.get_or_init(|| FIGfont::from_file("figfonts/lcd.flf").unwrap())
}

fn render_wpm(
    f: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
    stats: &SessionStats,
) {
    let wpm = stats.wpm_series.last().map_or(0.0, |(_, wpm)| *wpm);
    let widget = WpmWidget::new(wpm as u32, load_font());
    f.render_widget(widget, area);
}

fn render_text(
    f: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
    main: &PlayerState<'_>,
    opponents: &PlayerPool<'_>,
    styled_lines: &[StyledLine],
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Reset).fg(Color::White))
        .title(Span::styled(
            "Text",
            Style::default().add_modifier(Modifier::BOLD),
        ));

    let bg = get_theme(THEME).settings.background.unwrap();

    f.render_widget(
        DacttyloWidget::new(main, opponents, styled_lines)
            .block(block)
            .bg_color(Color::Rgb(bg.r, bg.g, bg.b)),
        area,
    );
}

fn render_chart(
    f: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
    stats: &SessionStats,
) {
    let data = stats
        .wpm_series
        .windows(30)
        .last()
        .unwrap_or_else(|| stats.wpm_series.as_slice());

    let last = data.last().map_or(0.0, |(secs, _)| *secs);
    let x_bounds = [last - 30.0, last];

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
                    "WPM",
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
                // .title("WPM")
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

// fn render_stats(
//     f: &mut Frame<CrosstermBackend<Stdout>>,
//     area: Rect,
//     stats: &SessionStats,
// ) {
//     let stats_fmt = format!("{}", stats);

//     let block = Block::default()
//         .borders(Borders::ALL)
//         .style(Style::default().bg(Color::Reset).fg(Color::White))
//         .title(Span::styled(
//             "Stats",
//             Style::default().add_modifier(Modifier::BOLD),
//         ));

//     let stats_widget = Paragraph::new(Text::from(stats_fmt))
//         .style(Style::default().bg(Color::Reset).fg(Color::White))
//         .block(block)
//         .alignment(Alignment::Left);

//     f.render_widget(stats_widget, area);
// }
