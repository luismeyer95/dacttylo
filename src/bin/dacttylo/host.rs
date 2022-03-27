use crate::{
    app::Game,
    common::{
        self, format_and_style, handle_wpm_tick, render, spawn_ticker,
        SessionEnd, SessionState,
    },
    protocol::{ProtocolClient, ProtocolCommand},
    report::{display_session_report, Ranking, SessionResult},
};
use chrono::{DateTime, Utc};
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
    network::{self, P2PEvent},
    record::manager::RecordManager,
    session::{event::SessionEvent, SessionClient, SessionCommand},
    stats::SessionStats,
    utils::{
        self,
        syntect::syntect_load_defaults,
        time::{datetime_in, wake_up},
        tui::{enter_tui_mode, leave_tui_mode},
        types::StyledLine,
    },
    widgets::{figtext::FigTextWidget, wpm::WpmWidget},
};
use dacttylo::{cli::HostOptions, utils::types::AsyncResult};
use figlet_rs::FIGfont;
use futures::Stream;
use libp2p::{identity, PeerId};
use once_cell::sync::OnceCell;
use std::{
    collections::{hash_map::Entry, HashMap},
    fs::read_to_string,
    io::Stdout,
    time::Duration,
};
use syntect::highlighting::Theme;
use tokio::sync::mpsc::Sender;
use tokio::{
    fs,
    io::{self, AsyncBufReadExt},
};
use tokio_stream::StreamExt;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, StyledGrapheme},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Frame, Terminal,
};

const THEME: &str = "Solarized (dark)";

pub async fn run_host_session(host_opts: HostOptions) -> AsyncResult<()> {
    println!("> Hosting as `{}`", host_opts.user);

    let text = fs::read_to_string(&host_opts.file).await?;

    let (peer_id, mut session_client, mut session_events) =
        connect_to_network().await?;

    let registered_users = take_registrations(
        peer_id,
        &mut session_client,
        &mut session_events,
        &text,
        &host_opts,
    )
    .await?;

    //////////////////
    let protocol_client = ProtocolClient::new(session_client);

    let game =
        init_game_state(&text, &registered_users, session_events, host_opts)
            .await?;

    let mut term = enter_tui_mode(std::io::stdout())?;
    let session_result = handle_events(
        &mut term,
        protocol_client,
        &registered_users,
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

pub async fn init_game_state<'t>(
    text: &'t str,
    registered_users: &HashMap<String, String>,
    session_events: impl Stream<Item = P2PEvent>,
    host_opts: HostOptions,
) -> AsyncResult<Game<'t, HostOptions>> {
    let (client, events) = configure_event_stream(session_events);

    let opponent_names: Vec<&str> =
        registered_users.iter().map(|(_, v)| v.as_ref()).collect();

    let main = PlayerState::new(host_opts.user.clone(), &text);
    let opponents = PlayerPool::new(&text).with_players(&opponent_names);

    Ok(Game::from(main, opponents, client, events, host_opts))
}

async fn handle_events(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    mut protocol_client: ProtocolClient,
    registered_users: &HashMap<String, String>,
    mut game: Game<'_, HostOptions>,
    text: &str,
) -> AsyncResult<Option<SessionResult>> {
    let styled_lines = format_and_style(text, &game.opts.file, game.theme)?;
    let mut stats = SessionStats::default();

    while let Some(event) = game.events.next().await {
        let session_state = handle_event(
            event,
            &mut protocol_client,
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
    protocol_client: &mut ProtocolClient,
    registered_users: &HashMap<String, String>,
    game: &mut Game<'_, HostOptions>,
    stats: &mut SessionStats,
) -> AsyncResult<SessionState> {
    match event {
        AppEvent::Term(e) => {
            return Ok(handle_term(e?, protocol_client, &mut game).await);
        }
        AppEvent::WpmTick => handle_wpm_tick(stats, &game.main),
        AppEvent::Session(e) => {
            handle_session_event(e, registered_users, game).ok();
        }
        _ => (),
    };

    Ok(SessionState::Ongoing)
}

fn handle_session_event(
    event: SessionEvent,
    registered_users: &HashMap<String, String>,
    game: &mut Game<HostOptions>,
) -> AsyncResult<()> {
    let SessionEvent::TopicMessage {
        source,
        topics,
        data,
    } = event;

    let (peer_id, cmd) =
        (source, bincode::deserialize::<SessionCommand>(&data)?);
    if let SessionCommand::Push(payload) = cmd {
        let username = registered_users
            .get(&peer_id.to_base58())
            .ok_or("session event origin user not found")?;

        let input_ch = std::str::from_utf8(&payload)?
            .chars()
            .nth(0)
            .ok_or("empty payload")?;

        game.opponents.process_input(username, input_ch)?;
    }

    Ok(())
}

async fn handle_term(
    term_event: crossterm::event::Event,
    protocol_client: &mut ProtocolClient,
    game: &mut Game<'_, HostOptions>,
) -> SessionState {
    if let Event::Key(event) = term_event {
        let KeyEvent { code, .. } = event;
        let c = match code {
            KeyCode::Esc => {
                protocol_client.publish(ProtocolCommand::Forfeit).await;
                return SessionState::End(SessionEnd::Quit);
            }
            KeyCode::Char(c) => Some(c),
            KeyCode::Enter => Some('\n'),
            KeyCode::Tab => Some('\t'),
            _ => None,
        };

        if let Some(c) = c {
            protocol_client.publish(ProtocolCommand::Input(c)).await;
            game.main.process_input(c);

            if game.main.is_done() && game.opponents.are_done() {
                return SessionState::End(SessionEnd::Finished);
            }
        }
    }

    SessionState::Ongoing
}

fn generate_session_result(
    stats: SessionStats,
    game: &Game<'_, HostOptions>,
) -> SessionResult {
}

pub fn configure_event_stream(
    session_stream: impl Stream<Item = P2PEvent>,
) -> (Sender<AppEvent>, EventAggregator<AppEvent>) {
    let (client, stream) = app_event::stream();
    spawn_ticker(client.clone());

    let term_io_stream = crossterm::event::EventStream::new();
    (
        client,
        aggregate!([stream, term_io_stream, session_stream] as AppEvent),
    )
}

async fn take_registrations(
    peer_id: PeerId,
    client: &mut SessionClient,
    events: &mut (impl Stream<Item = P2PEvent> + Unpin),
    text: &str,
    host_opts: &HostOptions,
) -> AsyncResult<HashMap<String, String>> {
    client.host_session(&host_opts.user, text.into()).await?;
    let mut registered_users: HashMap<String, String> = Default::default();
    registered_users.insert(peer_id.to_base58(), host_opts.user.clone());
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    loop {
        tokio::select! {
            // handle user input
            line = stdin.next_line() => {
                let _line = line?.expect("Standard input was closed");
                let date = lock_registrations(client, registered_users.clone()).await?;
                wake_up(Some(date)).await;
                return Ok(registered_users);
            }
            // handle session events
            event = events.next() => {
                let event = event.ok_or("event stream closed unexpectedly")?;
                let P2PEvent::TopicMessage {
                    source, data, ..
                } = event;
                let (peer_id, cmd) = (source, bincode::deserialize::<SessionCommand>(&data)?);

                if let SessionCommand::Register { user } = cmd {
                    registered_users.entry(peer_id.to_base58()).or_insert_with(|| {
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

async fn connect_to_network(
) -> AsyncResult<(PeerId, SessionClient, impl Stream<Item = P2PEvent>)> {
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());

    println!("Local peer id: {:?}", peer_id);

    let (online_client, online_events, task) =
        network::new(id_keys.clone()).await?;
    let online_client = SessionClient::new(online_client);

    tokio::spawn(task.run());

    Ok((peer_id, online_client, online_events))
}
