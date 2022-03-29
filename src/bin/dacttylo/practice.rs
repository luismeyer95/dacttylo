use crate::{common::*, report::*, AsyncResult};
use crossterm::event::{Event, KeyCode, KeyEvent};
use dacttylo::{
    app::{
        state::{PlayerPool, PlayerState},
        InputResult,
    },
    cli::{PracticeOptions, Save},
    events::AppEvent,
    game::game::Game,
    ghost::Ghost,
    record::manager::RecordManager,
    stats::GameStats,
    utils::tui::{enter_tui_mode, leave_tui_mode},
};
use std::{fs::read_to_string, io::Stdout};
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;
use tui::{backend::CrosstermBackend, Terminal};

const THEME: &str = "Solarized (dark)";

pub async fn run_practice_session(
    practice_opts: PracticeOptions,
) -> AsyncResult<()> {
    let text = read_to_string(&practice_opts.file)?;
    let game = Game::new(
        &text,
        if practice_opts.ghost { &["ghost"] } else { &[] },
        practice_opts,
        THEME,
    )?;

    let mut term = enter_tui_mode(std::io::stdout())?;
    let session_result = handle_events(&mut term, game, &text).await;

    let result = match session_result {
        Ok(Some(session_result)) => {
            display_session_report(&mut term, session_result).await
        }
        Ok(None) => Ok(()),
        Err(e) => Err(e),
    };

    leave_tui_mode(term)?;
    result
}

async fn handle_events(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    mut game: Game<'_, PracticeOptions>,
    text: &str,
) -> AsyncResult<Option<SessionResult>> {
    let styled_lines = format_and_style(text, &game.opts.file, &game.theme)?;
    let mut stats = GameStats::default();

    if game.opts.ghost {
        let mut ghost = initialize_ghost(text, game.client.clone())?;
        ghost.start().await?;
    }

    while let Some(event) = game.events.next().await {
        let session_state = handle_event(event, &mut game, &mut stats)?;

        if let SessionState::End(end) = session_state {
            if let SessionEnd::Finished = &end {
                update_record_state(text, &game.main, &game.opts)?;
                return Ok(Some(generate_session_result(
                    stats,
                    game.main,
                    game.opponents,
                    game.opts,
                )));
            } else {
                return Ok(None);
            }
        }

        render(term, &game, &styled_lines)?;
    }

    unreachable!();
}

fn handle_event<'t, O>(
    event: AppEvent,
    game: &mut Game<'t, O>,
    stats: &mut GameStats,
) -> AsyncResult<SessionState> {
    match event {
        AppEvent::Term(e) => return Ok(handle_term(e?, &mut game.main)),
        AppEvent::GhostInput(c) => handle_ghost_input(c, &mut game.opponents),
        AppEvent::WpmTick => handle_wpm_tick(stats, &game.main),
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
            main.process_input(c);
            if main.is_done() {
                return SessionState::End(SessionEnd::Finished);
            }
        }
    }

    SessionState::Ongoing
}

fn handle_ghost_input(input: InputResult, opponents: &mut PlayerPool) {
    if let InputResult::Correct = input {
        opponents.advance_player("ghost").unwrap();
    }
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

fn generate_session_result(
    stats: GameStats,
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
        let (spot, ranked): (usize, Vec<&str>) =
            if opponents.player("ghost").unwrap().is_done() {
                (1, vec!["ghost", main.name.as_ref()])
            } else {
                (0, vec![main.name.as_ref(), "ghost"])
            };

        SessionResult {
            stats,
            ranking: Some(Ranking {
                spot,
                names: ranked.iter().map(|&s| s.to_string()).collect(),
            }),
        }
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
