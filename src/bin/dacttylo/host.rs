use crate::{
    common::*,
    protocol::ProtocolCommand,
    report::{display_session_report, SessionResult},
};
use bincode::{deserialize, serialize};
use chrono::{DateTime, Utc};
use crossterm::event::{Event, KeyCode, KeyEvent};
use dacttylo::{
    aggregate,
    app::state::{PlayerPool, PlayerState},
    cli::base_opts::BaseOpts,
    events::{app_event, AppEvent, EventAggregator},
    game::game::Game,
    network::{self, P2PEvent},
    session::{
        self, event::SessionEvent, session_handle::SessionHandle,
        SessionClient, SessionCommand,
    },
    stats::SessionStats,
    utils::{
        time::{datetime_in, wake_up},
        tui::{enter_tui_mode, leave_tui_mode},
    },
};
use dacttylo::{cli::HostOptions, utils::types::AsyncResult};
use futures::Stream;
use std::{collections::HashMap, io::Stdout};
use tokio::sync::mpsc::Sender;
use tokio::{
    fs,
    io::{self, AsyncBufReadExt},
};
use tokio_stream::StreamExt;
use tui::{backend::CrosstermBackend, Terminal};

pub async fn run_host_session(host_opts: HostOptions) -> AsyncResult<()> {
    println!("> Hosting as `{}`", host_opts.user);

    let text = fs::read_to_string(&host_opts.file).await?;

    let mut session = session::new().await?;

    let (start_date, mut registered_users) =
        take_registrations(&mut session, &text, &host_opts).await?;

    //////////////////
    let opponent_names: Vec<&str> =
        registered_users.iter().map(|(_, v)| v.as_ref()).collect();

    let game = init_game_state(&text, &opponent_names, host_opts)?;

    let mut term = enter_tui_mode(std::io::stdout())?;
    let session_result = handle_events(
        &mut term,
        session_client,
        &mut registered_users,
        game,
        &text,
    )
    .await;

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

pub fn init_game_state<'t, O: BaseOpts>(
    text: &'t str,
    opponent_names: &[&str],
    opts: O,
) -> AsyncResult<Game<'t, O>> {
    let (client, events) = configure_event_stream();

    let username = opts.get_username().unwrap_or("you");

    let main = PlayerState::new(username.to_owned(), &text);
    let opponents = PlayerPool::new(&text).with_players(&opponent_names);

    Ok(Game::from(main, opponents, client, events, opts))
}

async fn handle_events(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    mut session_client: SessionClient,
    registered_users: &mut HashMap<String, String>,
    mut game: Game<'_, HostOptions>,
    text: &str,
) -> AsyncResult<Option<SessionResult>> {
    let styled_lines = format_and_style(text, &game.opts.file, game.theme)?;
    let mut stats = SessionStats::default();

    while let Some(event) = game.events.next().await {
        let session_state = handle_event(
            event,
            &mut session_client,
            registered_users,
            &mut game,
            &mut stats,
        )
        .await?;

        if let SessionState::End(end) = session_state {
            if let SessionEnd::Finished = &end {
                return Ok(Some(generate_session_result(stats, &game)));
            } else {
                return Ok(None);
            }
        }

        render(term, &game, &stats, &styled_lines)?;
    }

    unreachable!();
}

async fn handle_event(
    event: AppEvent,
    session_client: &mut SessionClient,
    registered_users: &mut HashMap<String, String>,
    game: &mut Game<'_, HostOptions>,
    stats: &mut SessionStats,
) -> AsyncResult<SessionState> {
    match event {
        AppEvent::Term(e) => {
            return handle_term(e?, session_client, game).await;
        }
        AppEvent::WpmTick => handle_wpm_tick(stats, &game.main),
        AppEvent::Session(e) => {
            return handle_session_event(e, registered_users, game);
        }
        _ => (),
    };

    Ok(SessionState::Ongoing)
}

fn handle_session_event(
    event: SessionEvent,
    registered_users: &mut HashMap<String, String>,
    game: &mut Game<HostOptions>,
) -> AsyncResult<SessionState> {
    let SessionEvent { peer_id, cmd } = event;

    if let SessionCommand::Push(payload) = cmd {
        let cmd = deserialize::<ProtocolCommand>(&payload)?;

        match cmd {
            ProtocolCommand::Input(ch) => {
                let username = registered_users
                    .get(&peer_id)
                    .ok_or("session event origin user not found")?;

                game.opponents.process_input(username, ch)?;

                if game.main.is_done() && game.opponents.are_done() {
                    return Ok(SessionState::End(SessionEnd::Finished));
                }
            }
            ProtocolCommand::Forfeit => {
                registered_users.remove(&peer_id);
            }
        }
    }

    Ok(SessionState::Ongoing)
}

async fn handle_term(
    term_event: crossterm::event::Event,
    session_client: &mut SessionClient,
    game: &mut Game<'_, HostOptions>,
) -> AsyncResult<SessionState> {
    if let Event::Key(event) = term_event {
        let KeyEvent { code, .. } = event;
        let c = match code {
            KeyCode::Esc => {
                let serial = serialize(&ProtocolCommand::Forfeit)?;
                session_client.publish(SessionCommand::Push(serial)).await;
                return Ok(SessionState::End(SessionEnd::Quit));
            }
            KeyCode::Char(c) => Some(c),
            KeyCode::Enter => Some('\n'),
            KeyCode::Tab => Some('\t'),
            _ => None,
        };

        if let Some(c) = c {
            let serial = serialize(&ProtocolCommand::Input(c))?;
            session_client.publish(SessionCommand::Push(serial)).await;

            game.main.process_input(c);

            if game.main.is_done() && game.opponents.are_done() {
                return Ok(SessionState::End(SessionEnd::Finished));
            }
        }
    }

    Ok(SessionState::Ongoing)
}

fn generate_session_result(
    stats: SessionStats,
    game: &Game<'_, HostOptions>,
) -> SessionResult {
    todo!()
}

async fn take_registrations(
    session: &mut SessionHandle,
    text: &str,
    host_opts: &HostOptions,
) -> AsyncResult<(DateTime<Utc>, HashMap<String, String>)> {
    session
        .client
        .host_session(&host_opts.user, text.into())
        .await?;
    let mut registered_users: HashMap<String, String> = Default::default();
    registered_users
        .insert(session.peer_id.to_base58(), host_opts.user.clone());
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    loop {
        tokio::select! {
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

// async fn connect_to_network(
// ) -> AsyncResult<(PeerId, SessionClient, impl Stream<Item = P2PEvent>)> {
//     let id_keys = identity::Keypair::generate_ed25519();
//     let peer_id = PeerId::from(id_keys.public());

//     println!("Local peer id: {:?}", peer_id);

//     let (online_client, online_events, task) =
//         network::new(id_keys.clone()).await?;

//     tokio::spawn(task.run());

//     Ok((peer_id, SessionClient::new(online_client), online_events))
// }
