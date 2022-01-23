use crate::AsyncResult;
use crossterm::event::{Event, KeyCode, KeyEvent};
use dacttylo::{
    aggregate,
    app::{state::DacttyloGameState, widget::DacttyloWidget},
    events::{
        ticker::{self, TickerClient},
        AppEvent, EventAggregator,
    },
    highlighting::SyntectHighlighter,
    utils::{
        helpers::get_extension_from_filename,
        tui::{enter_tui_mode, leave_tui_mode},
        types::Action,
    },
};
use std::io::Stdout;
use tokio_stream::StreamExt;
use tui::{backend::CrosstermBackend, Terminal};

pub async fn init_practice_session(practice_file: String) -> AsyncResult<()> {
    // TODO: remove duplicate, this is for erroring out early (before tui mode)
    SyntectHighlighter::new()
        .file(Some(&practice_file))?
        .theme("base16-mocha.dark");

    let mut term = enter_tui_mode(std::io::stdout())?;

    let (ticker_client, ticker_stream) = ticker::new();
    let term_io_stream = crossterm::event::EventStream::new();
    let global_stream = aggregate!([ticker_stream, term_io_stream] as AppEvent);

    run_practice_session(
        &mut term,
        global_stream,
        ticker_client,
        practice_file,
    )
    .await?;

    leave_tui_mode(term)?;

    Ok(())
}

async fn run_practice_session(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    mut global_stream: EventAggregator<AppEvent>,
    ticker_client: TickerClient,
    practice_file: String,
) -> AsyncResult<()> {
    let text_contents = std::fs::read_to_string(&practice_file)?;

    let hl_builder = SyntectHighlighter::new()
        .file(practice_file.into())?
        .theme("base16-mocha.dark");

    let mut game_state = DacttyloGameState::new("Luis", &text_contents)
        .with_players(&["Agathe"]);
    ticker_client.tick().await?;

    while let Some(event) = global_stream.next().await {
        match event {
            AppEvent::Tick => {}
            AppEvent::Session(_session_event) => {}
            AppEvent::Term(e) => {
                if let Action::Quit =
                    handle_term_event(e?, &mut game_state).await
                {
                    return Ok(());
                }
            }
        }

        term.draw(|f| {
            f.render_widget(
                DacttyloWidget::new(&game_state)
                    .highlighter(&hl_builder.clone().build().unwrap()),
                f.size(),
            );
        })?;
    }
    Ok(())
}

async fn handle_term_event(
    term_event: crossterm::event::Event,
    game_state: &mut DacttyloGameState<'_>,
) -> Action {
    if let Event::Key(event) = term_event {
        let KeyEvent { code, .. } = event;
        match code {
            KeyCode::Char(c) => {
                game_state.process_input("Luis", c).unwrap();
            }
            KeyCode::Enter => {
                game_state.process_input("Luis", '\n').unwrap();
            }
            KeyCode::Tab => {
                game_state.process_input("Luis", '\t').unwrap();
            }
            KeyCode::Esc => return Action::Quit,
            _ => {}
        };
    }

    Action::Ok
}
