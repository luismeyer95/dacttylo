use crate::AsyncResult;
use crossterm::event::{Event, KeyCode, KeyEvent};
use dacttylo::{
    aggregate,
    app::{state::PlayerPool, widget::DacttyloWidget, InputResult, Progress},
    events::{app_event, AppEvent, EventAggregator},
    ghost::Ghost,
    highlighting::{Highlighter, SyntectHighlighter},
    input::record::{InputRecorder, RecordManager},
    utils::{
        tui::{enter_tui_mode, leave_tui_mode},
        types::Action,
    },
};
use std::io::Stdout;
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;
use tui::{backend::CrosstermBackend, style::Style, Terminal};

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
    let mut game_state = initialize_player_state(&text);

    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let styled_lines = apply_highlighting(&lines, &file)?;

    let (client, mut events) = configure_event_stream();
    let mut ghost = initialize_ghost(&text, client.clone())?;

    client.send(AppEvent::Tick).await?;
    ghost.start().await?;

    let mut term = enter_tui_mode(std::io::stdout())?;
    let session_result =
        handle_events(&mut term, client, events, game_state, styled_lines)
            .await;
    leave_tui_mode(term)?;

    /// display session results
    Ok(())
}

pub fn initialize_player_state<'txt>(text: &'txt str) -> PlayerPool<'txt> {
    PlayerPool::new("self", &text).with_players(&["ghost"])
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
        RecordManager::mount_dir("records")?.load_from_contents(&text)?;
    Ok(Ghost::new(input_record, client.clone()))
}

async fn handle_events(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    client: Sender<AppEvent>,
    mut events: EventAggregator<AppEvent>,
    mut game_state: PlayerPool<'_>,
    styled_lines: Vec<Vec<(&str, Style)>>,
) -> AsyncResult<SessionEnd> {
    let mut recorder = InputRecorder::new();

    while let Some(event) = events.next().await {
        let session_state = match event {
            AppEvent::Term(e) => {
                handle_term_event(e?, &mut game_state, &mut recorder).await
            }
            AppEvent::GhostInput(c) => handle_ghost_input(c, &mut game_state),
            _ => SessionState::Ongoing,
        };

        if let SessionState::End(end) = session_state {
            return Ok(end);
        }

        render_text(term, &game_state, styled_lines.clone());
    }

    unreachable!();
}

fn handle_ghost_input(c: char, game_state: &mut PlayerPool) -> SessionState {
    let input_result = game_state.process_input("ghost", c).unwrap();

    if let InputResult::Correct(Progress::Finished) = input_result {
        SessionState::End(SessionEnd::Finished(SessionResult))
    } else {
        SessionState::Ongoing
    }
}

fn render_text(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    game_state: &PlayerPool<'_>,
    styled_lines: Vec<Vec<(&str, Style)>>,
) -> AsyncResult<()> {
    term.draw(|f| {
        f.render_widget(
            DacttyloWidget::new(&game_state).highlighted_content(styled_lines),
            f.size(),
        );
    })?;

    Ok(())
}

async fn handle_term_event(
    term_event: crossterm::event::Event,
    game_state: &mut PlayerPool<'_>,
    recorder: &mut InputRecorder,
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
            recorder.push(c);
            let input_result = game_state.process_input("self", c).unwrap();

            if let InputResult::Correct(Progress::Finished) = input_result {
                // let manager = RecordManager::mount_dir("records").unwrap();
                // manager.save(game_state.text(), recorder.record()).unwrap();
                return SessionState::End(SessionEnd::Quit);
            }
        }
    }

    SessionState::Ongoing
}
