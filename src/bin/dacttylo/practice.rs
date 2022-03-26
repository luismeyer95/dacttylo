use crate::{
    report::{display_session_report, Ranking, SessionResult},
    AsyncResult,
};
use crossterm::event::{Event, KeyCode, KeyEvent};
use dacttylo::{
    aggregate,
    app::{
        state::{PlayerPool, PlayerState},
        widget::DacttyloWidget,
        InputResult, Progress,
    },
    cli::{PracticeOptions, Save},
    events::{app_event, AppEvent, EventAggregator},
    ghost::Ghost,
    highlighting::{Highlighter, SyntectHighlighter},
    record::manager::RecordManager,
    stats::SessionStats,
    utils::{
        self,
        syntect::syntect_load_defaults,
        tui::{enter_tui_mode, leave_tui_mode},
        types::StyledLine,
    },
    widgets::wpm::WpmWidget,
};
use figlet_rs::FIGfont;
use once_cell::sync::OnceCell;
use std::{fs::read_to_string, io::Stdout, time::Duration};
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

enum SessionState {
    Ongoing,
    End(SessionEnd),
}

enum SessionEnd {
    Finished,
    Quit,
}

pub async fn run_practice_session(
    practice_opts: PracticeOptions,
) -> AsyncResult<()> {
    let (client, events) = configure_event_stream();
    spawn_ticker(client.clone());

    let mut term = enter_tui_mode(std::io::stdout())?;
    let session_result =
        handle_events(&mut term, client, events, practice_opts).await;

    match session_result {
        Ok(None) => {
            leave_tui_mode(term)?;
            Ok(())
        }
        Ok(Some(session_result)) => {
            display_session_report(&mut term, session_result.clone()).await;
            leave_tui_mode(term)?;
            println!("{:?}", session_result);
            Ok(())
        }
        Err(e) => {
            leave_tui_mode(term)?;
            Err(e)
        }
    }
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

pub fn initialize_ghost(
    text: &str,
    client: Sender<AppEvent>,
) -> AsyncResult<Ghost> {
    let input_record = RecordManager::mount_dir("records")?
        .load_from_contents(text)
        .map_err(|_| "no ghost record found for this file")?;
    Ok(Ghost::new(input_record, client))
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

pub fn format_and_style<'t>(
    text: &'t str,
    practice_opts: &PracticeOptions,
) -> AsyncResult<Vec<Vec<StyledGrapheme<'t>>>> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();

    let hl = SyntectHighlighter::new()
        .from_file((&practice_opts.file).into())?
        .theme(get_theme(THEME))
        .build()?;

    Ok(hl.highlight(&lines))
}

async fn handle_events(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    client: Sender<AppEvent>,
    mut events: EventAggregator<AppEvent>,
    mut practice_opts: PracticeOptions,
) -> AsyncResult<Option<SessionResult>> {
    let text = read_to_string(&practice_opts.file)?;
    let styled_lines = format_and_style(&text, &practice_opts)?;

    let username = practice_opts
        .username
        .take()
        .unwrap_or_else(|| "you".into());
    let mut main = PlayerState::new(username, &text);
    let mut opponents = if practice_opts.ghost {
        let mut ghost = initialize_ghost(&text, client.clone())?;
        ghost.start().await?;
        PlayerPool::new(&text).with_players(&["ghost"])
    } else {
        PlayerPool::new(&text)
    };

    let mut stats = SessionStats::default();

    while let Some(event) = events.next().await {
        let session_state =
            handle_event(event, &mut main, &mut opponents, &mut stats)?;

        if let SessionState::End(end) = session_state {
            if let SessionEnd::Finished = &end {
                update_record_state(&text, &main, &practice_opts)?;
                return Ok(Some(generate_session_result(
                    stats,
                    main,
                    opponents,
                    practice_opts,
                )));
            } else {
                return Ok(None);
            }
        }

        render(term, &main, &opponents, &stats, &styled_lines)?;
    }

    unreachable!();
}

fn generate_session_result(
    stats: SessionStats,
    main: PlayerState,
    opponents: PlayerPool,
    practice_opts: PracticeOptions,
) -> SessionResult {
    if !practice_opts.ghost {
        SessionResult {
            stats,
            ranking: None,
        }
    } else {
        let ghost_progress = opponents.player("ghost").unwrap().get_progress();
        let (spot, names): (usize, Vec<&str>) =
            if ghost_progress == Progress::Finished {
                (1, vec!["ghost", main.name.as_ref()])
            } else {
                (0, vec![main.name.as_ref(), "ghost"])
            };

        SessionResult {
            stats,
            ranking: Some(Ranking {
                spot,
                names: names.iter().map(|&s| s.to_string()).collect(),
            }),
        }
    }
}

fn handle_event(
    event: AppEvent,
    main: &mut PlayerState,
    opponents: &mut PlayerPool,
    stats: &mut SessionStats,
) -> AsyncResult<SessionState> {
    match event {
        AppEvent::Term(e) => return Ok(handle_term(e?, main)),
        AppEvent::GhostInput(c) => handle_ghost_input(c, opponents),
        AppEvent::WpmTick => handle_wpm_tick(stats, main),
        _ => (),
    };

    Ok(SessionState::Ongoing)
}

fn handle_term(
    term_event: crossterm::event::Event,
    main: &mut PlayerState<'_>,
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
            if let InputResult::Correct(Progress::Finished) =
                main.process_input(c).unwrap()
            {
                return SessionState::End(SessionEnd::Finished);
            }
        }
    }

    SessionState::Ongoing
}

fn handle_wpm_tick(stats: &mut SessionStats, main: &PlayerState) {
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

fn handle_ghost_input(input: InputResult, opponents: &mut PlayerPool) {
    if let InputResult::Correct(_) = input {
        opponents.advance_player("ghost").unwrap();
    }
}

fn update_record_state(
    text: &str,
    main: &PlayerState,
    practice_opts: &PracticeOptions,
) -> AsyncResult<()> {
    if let Some(save) = practice_opts.save {
        let manager = RecordManager::mount_dir("records")?;
        let record = &main.recorder.record();

        match save {
            Save::Override => manager.save(text, record)?,

            Save::Best => {
                if let Ok(old_record) = manager.load_from_contents(text) {
                    let (old_elapsed, _) = old_record.inputs.last().unwrap();
                    let (current_elapsed, _) = record.inputs.last().unwrap();

                    if current_elapsed.duration < old_elapsed.duration {
                        manager.save(text, record)?;
                    }
                } else {
                    manager.save(text, record)?;
                }
            }
        }
    }

    Ok(())
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

pub fn load_wpm_font() -> &'static FIGfont {
    static FONT: OnceCell<FIGfont> = OnceCell::new();
    FONT.get_or_init(|| {
        let bytes = include_bytes!("figfonts/lcd.flf");
        let s = std::str::from_utf8(bytes).unwrap();
        FIGfont::from_content(s).unwrap()
    })
}

fn render_wpm(
    f: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
    stats: &SessionStats,
) {
    let wpm = stats.wpm_series.last().map_or(0.0, |(_, wpm)| *wpm);
    let widget = WpmWidget::new(wpm as u32, load_wpm_font());
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
        .style(Style::default().bg(Color::Reset).fg(Color::White));

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
