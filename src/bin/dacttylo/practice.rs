use crate::AsyncResult;
use crossterm::event::{Event, KeyCode, KeyEvent};
use dacttylo::{
    aggregate,
    app::{
        state::{PlayerPool, PlayerState},
        widget::DacttyloWidget,
        InputResult, Progress,
    },
    events::{app_event, AppEvent, EventAggregator},
    ghost::Ghost,
    highlighting::{Highlighter, SyntectHighlighter},
    record::{manager::RecordManager, recorder::InputResultRecorder},
    stats::SessionStats,
    utils::tui::{enter_tui_mode, leave_tui_mode},
};
use std::{io::Stdout, time::Duration};
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

pub struct SessionResult;

enum SessionState {
    Ongoing,
    End(SessionEnd),
}

enum SessionEnd {
    Finished(SessionResult),
    Quit,
}

pub async fn init_practice_session(practice_file: String) -> AsyncResult<()> {
    let result = run_practice_session(practice_file).await;

    result
}

async fn run_practice_session(file: String) -> AsyncResult<()> {
    let text = std::fs::read_to_string(&file)?;
    let (main, opponents) = initialize_players(&text);

    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let styled_lines = apply_highlighting(&lines, &file)?;

    let (client, events) = configure_event_stream();
    // let mut ghost = initialize_ghost(&text, client.clone())?;

    client.send(AppEvent::Tick).await?;
    // ghost.start().await?;

    let mut term = enter_tui_mode(std::io::stdout())?;
    let session_result =
        handle_events(&mut term, main, opponents, events, client, styled_lines)
            .await;
    leave_tui_mode(term)?;

    // display session results
    Ok(())
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

pub fn apply_highlighting<'t>(
    lines: &[&'t str],
    file: &str,
) -> AsyncResult<Vec<Vec<(&'t str, Style)>>> {
    let hl = SyntectHighlighter::new()
        .from_file(file.into())?
        .theme("base16-mocha.dark")
        .build()?;

    Ok(hl.highlight(lines.as_ref()))
}

pub fn initialize_ghost(
    text: &str,
    client: Sender<AppEvent>,
) -> AsyncResult<Ghost> {
    let input_record =
        RecordManager::mount_dir("records")?.load_from_contents(text)?;
    Ok(Ghost::new(input_record, client))
}

async fn handle_events(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    mut main: PlayerState<'_>,
    mut opponents: PlayerPool<'_>,
    mut events: EventAggregator<AppEvent>,
    client: Sender<AppEvent>,
    styled_lines: Vec<Vec<(&str, Style)>>,
) -> AsyncResult<SessionEnd> {
    let mut recorder = InputResultRecorder::new();
    let mut stats = SessionStats::default();

    let wpm_client = client.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            wpm_client.send(AppEvent::WpmTick).await.unwrap();
        }
    });

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

        render(term, &main, &opponents, &stats, styled_lines.clone())?;
    }

    unreachable!();
}

fn handle_wpm_tick(
    stats: &mut SessionStats,
    recorder: &InputResultRecorder,
) -> SessionState {
    let record = recorder.record();
    stats.wpm = record.wpm_at(Duration::from_secs(4), recorder.elapsed());
    stats.average_wpm = record.average_wpm(recorder.elapsed());
    stats.top_wpm = f64::max(stats.wpm, stats.top_wpm);
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
    styled_lines: Vec<Vec<(&str, Style)>>,
) -> AsyncResult<()> {
    term.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(
                [Constraint::Length(7), Constraint::Percentage(80)].as_ref(),
            )
            .split(f.size());

        render_stats(f, chunks[0], stats);
        render_text(f, chunks[1], main, opponents, styled_lines);
    })?;

    Ok(())
}

fn render_stats(
    f: &mut Frame<CrosstermBackend<Stdout>>,
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

    let stats_widget = Paragraph::new(Text::from(stats_fmt))
        .style(Style::default().bg(Color::Reset).fg(Color::White))
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(stats_widget, area);
}

fn render_text(
    f: &mut Frame<CrosstermBackend<Stdout>>,
    area: Rect,
    main: &PlayerState<'_>,
    opponents: &PlayerPool<'_>,
    styled_lines: Vec<Vec<(&str, Style)>>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Reset).fg(Color::White))
        .title(Span::styled(
            "Text",
            Style::default().add_modifier(Modifier::BOLD),
        ));

    f.render_widget(
        DacttyloWidget::new(main, opponents)
            .highlighted_content(styled_lines)
            .block(block),
        area,
    );
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
