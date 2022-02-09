use crate::AsyncResult;
use crossterm::event::{Event, KeyCode, KeyEvent};
use dacttylo::{
    aggregate,
    app::{
        state::DacttyloGameState, widget::DacttyloWidget, InputResult, Progress,
    },
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
use tokio_stream::StreamExt;
use tui::{backend::CrosstermBackend, Terminal};

pub async fn init_practice_session(practice_file: String) -> AsyncResult<()> {
    let mut term = enter_tui_mode(std::io::stdout())?;

    let result = run_practice_session(&mut term, practice_file).await;

    leave_tui_mode(term)?;

    result
}

async fn run_practice_session(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    practice_file: String,
) -> AsyncResult<()> {
    let text_contents = std::fs::read_to_string(&practice_file)?;

    // setup event stream
    // let (mut ghost_client, ghost_stream) = ghost::new(input_record);
    let (ticker_client, ticker_stream) = app_event::stream();
    let term_io_stream = crossterm::event::EventStream::new();
    let mut global_stream =
        aggregate!([ticker_stream, term_io_stream] as AppEvent);

    // initialize game state
    let mut game_state = DacttyloGameState::new("Luis", &text_contents)
        .with_players(&["Agathe"]);
    ticker_client.send(AppEvent::Tick).await?;

    // highlight syntax
    let text_lines: Vec<&str> =
        game_state.text().split_inclusive('\n').collect();
    let hl = SyntectHighlighter::new()
        .file(practice_file.into())?
        .theme("base16-mocha.dark")
        .build()?;
    let styled_lines = hl.highlight(text_lines.as_ref());

    let mut recorder = InputRecorder::new();

    // load up ghost
    let input_record = RecordManager::mount_dir("records")?
        .load_from_contents(&text_contents)?;
    let mut ghost = Ghost::new(input_record, ticker_client.clone());
    ghost.start().await?;

    while let Some(event) = global_stream.next().await {
        match event {
            AppEvent::Tick => {}
            AppEvent::Term(e) => {
                if let Action::Quit =
                    handle_term_event(e?, &mut game_state, &mut recorder).await
                {
                    return Ok(());
                }
            }
            AppEvent::GhostInput(c) => {
                let input_result =
                    game_state.process_input("Agathe", c).unwrap();
            }
            _ => {}
        }

        term.draw(|f| {
            f.render_widget(
                DacttyloWidget::new(&game_state)
                    .highlighted_content(styled_lines.clone()),
                f.size(),
            );
        })?;
    }
    Ok(())
}

async fn handle_term_event(
    term_event: crossterm::event::Event,
    game_state: &mut DacttyloGameState<'_>,
    recorder: &mut InputRecorder,
) -> Action {
    if let Event::Key(event) = term_event {
        let KeyEvent { code, .. } = event;
        let c = match code {
            KeyCode::Esc => return Action::Quit,
            KeyCode::Char(c) => Some(c),
            KeyCode::Enter => Some('\n'),
            KeyCode::Tab => Some('\t'),
            _ => None,
        };

        if let Some(c) = c {
            recorder.push(c);
            let input_result = game_state.process_input("Luis", c).unwrap();

            if let InputResult::Correct(Progress::Finished) = input_result {
                // let manager = RecordManager::mount_dir("records").unwrap();
                // manager.save(game_state.text(), recorder.record()).unwrap();
                return Action::Quit;
            }
        }
    }

    Action::Ok
}
