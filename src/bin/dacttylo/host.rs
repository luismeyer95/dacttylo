use crate::{
    common::*,
    protocol::ProtocolCommand,
    report::{display_session_report, Ranking, SessionResult},
};
use bincode::{deserialize, serialize};
use chrono::{DateTime, Utc};
use crossterm::event::{Event, KeyCode, KeyEvent};
use dacttylo::{cli::HostOptions, utils::types::AsyncResult};
use dacttylo::{
    events::AppEvent,
    game::{game::Game, online_game::OnlineGame},
    session::{
        self, event::SessionEvent, session_handle::SessionHandle,
        SessionClient, SessionCommand,
    },
    utils::{
        time::{datetime_in, wake_up},
        tui::{enter_tui_mode, leave_tui_mode},
    },
};
use std::{collections::HashMap, io::Stdout, iter, time::Duration};
use tokio::{
    fs,
    io::{self, AsyncBufReadExt},
    select,
};
use tokio_stream::StreamExt;
use tui::{backend::CrosstermBackend, Terminal};

pub async fn run_host_session(host_opts: HostOptions) -> AsyncResult<()> {
    println!("> Hosting as `{}`", host_opts.username);

    let text = fs::read_to_string(&host_opts.file).await?;

    let mut session = session::new().await?;
    println!("Local peer id: {:?}", session.peer_id);

    let (start_date, registered_users) =
        take_registrations(&mut session, &text, &host_opts).await?;

    let opponent_names: Vec<&str> =
        registered_users.iter().map(|(_, v)| v.as_ref()).collect();
    let game =
        OnlineGame::new(session, Game::new(&text, &opponent_names, host_opts)?);

    wake_up(Some(start_date)).await;

    let mut term = enter_tui_mode(std::io::stdout())?;
    let session_result =
        handle_events(&mut term, registered_users, game, &text).await;

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
    mut registered_users: HashMap<String, String>,
    mut app: OnlineGame<'_, HostOptions>,
    text: &str,
) -> AsyncResult<Option<SessionResult>> {
    let styled_lines =
        format_and_style(text, &app.game.opts.file, app.game.theme)?;

    loop {
        let event = select! {
            Some(event) = app.game.events.next() => event,
            Some(event) = app.session.events.next() => event.into()
        };

        let session_state =
            handle_event(event, &mut registered_users, &mut app).await?;

        if let SessionState::End(end) = session_state {
            if let SessionEnd::Finished = &end {
                return Ok(Some(generate_session_result(app.game)));
            } else {
                return Ok(None);
            }
        }

        render(term, &app.game, &styled_lines)?;
    }
}

async fn handle_event(
    event: AppEvent,
    registered_users: &mut HashMap<String, String>,
    app: &mut OnlineGame<'_, HostOptions>,
) -> AsyncResult<SessionState> {
    match event {
        AppEvent::Term(e) => handle_term(e?, app).await,
        AppEvent::Session(e) => {
            handle_session_event(e, registered_users, &mut app.game)
        }
        AppEvent::WpmTick => {
            handle_wpm_tick(&mut app.game.stats, &app.game.main);
            Ok(SessionState::Ongoing)
        }
        _ => Ok(SessionState::Ongoing),
    }
}

fn handle_session_event(
    event: SessionEvent,
    registered_users: &mut HashMap<String, String>,
    game: &mut Game<HostOptions>,
) -> AsyncResult<SessionState> {
    let SessionEvent { peer_id, cmd } = event;

    if let SessionCommand::Push(payload) = cmd {
        let cmd = deserialize::<ProtocolCommand>(&payload)?;
        let username = registered_users
            .get(&peer_id)
            .ok_or("session event origin user not found")?;

        match cmd {
            ProtocolCommand::Input(ch) => {
                game.opponents.process_input(username, ch)?;
                if game.main.is_done() && game.opponents.are_done() {
                    return Ok(SessionState::End(SessionEnd::Finished));
                }
            }
            ProtocolCommand::Forfeit => {
                game.opponents.remove(username);
                registered_users.remove(&peer_id);
            }
        }
    }

    Ok(SessionState::Ongoing)
}

async fn handle_term(
    term_event: crossterm::event::Event,
    app: &mut OnlineGame<'_, HostOptions>,
) -> AsyncResult<SessionState> {
    let client = &mut app.session.client;

    if let Event::Key(event) = term_event {
        let KeyEvent { code, .. } = event;
        let c = match code {
            KeyCode::Esc => {
                let serial = serialize(&ProtocolCommand::Forfeit)?;
                client.publish(SessionCommand::Push(serial)).await.unwrap();
                return Ok(SessionState::End(SessionEnd::Quit));
            }
            KeyCode::Char(c) => Some(c),
            KeyCode::Enter => Some('\n'),
            KeyCode::Tab => Some('\t'),
            _ => None,
        };

        if let Some(c) = c {
            let serial = serialize(&ProtocolCommand::Input(c))?;
            client.publish(SessionCommand::Push(serial)).await.unwrap();

            app.game.main.process_input(c);

            if app.game.main.is_done() && app.game.opponents.are_done() {
                return Ok(SessionState::End(SessionEnd::Finished));
            }
        }
    }

    Ok(SessionState::Ongoing)
}

fn generate_session_result(game: Game<'_, HostOptions>) -> SessionResult {
    let mut ranking = game
        .opponents
        .players()
        .iter()
        .chain(iter::once((game.main.name(), &game.main)))
        .filter_map(|(name, state)| {
            if state.is_done() {
                let completion_time =
                    &state.recorder.record().inputs.last().unwrap().0;
                Some((name.as_ref(), completion_time.duration))
            } else {
                None
            }
        })
        .collect::<Vec<(&str, Duration)>>();

    ranking.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
    let spot = ranking
        .iter()
        .position(|(name, _)| name == game.main.name())
        .unwrap();
    let ranking = ranking
        .into_iter()
        .map(|(name, _)| name.to_owned())
        .collect();

    SessionResult {
        stats: game.stats,
        ranking: Some(Ranking {
            spot,
            names: ranking,
        }),
    }
}

async fn take_registrations(
    session: &mut SessionHandle,
    text: &str,
    host_opts: &HostOptions,
) -> AsyncResult<(DateTime<Utc>, HashMap<String, String>)> {
    session
        .client
        .host_session(&host_opts.username, text.into())
        .await?;
    let mut registered_users: HashMap<String, String> = Default::default();
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    loop {
        select! {
            // handle user input
            line = stdin.next_line() => {
                let _line = line?.expect("Standard input was closed");
                let date = lock_registrations(&mut session.client, registered_users.clone()).await?;
                return Ok((date, registered_users));
            }
            // handle session events
            event = session.events.next() => {
                let event = event.ok_or("event stream closed unexpectedly")?;
                let SessionEvent {
                    peer_id, cmd
                } = event.into();

                if let SessionCommand::Register { user } = cmd {
                    registered_users.entry(peer_id).or_insert_with(|| {
                        println!("Registering user `{}`", user);
                        user
                    });
                };
            }
        };
    }
}

async fn lock_registrations(
    client: &mut SessionClient,
    registered_users: HashMap<String, String>,
) -> AsyncResult<DateTime<Utc>> {
    let date = datetime_in(chrono::Duration::seconds(3)).unwrap();
    let lock_cmd = SessionCommand::LockSession {
        registered_users: registered_users.clone(),
        session_start: date.to_string(),
    };

    println!("Locking session...");
    client.publish(lock_cmd).await?;
    println!("Session locked, starting soon :)");

    Ok(date)
}
