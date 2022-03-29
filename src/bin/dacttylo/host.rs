use crate::{
    common::*,
    protocol::{DacttyloCommand, DacttyloMetadata},
    report::{display_session_report, Ranking, SessionResult},
};
use bincode::{deserialize, serialize};
use chrono::{DateTime, Utc};
use crossterm::event::{Event, KeyCode, KeyEvent};
use dacttylo::{
    cli::HostOptions,
    highlighting::{Highlighter, SyntectHighlighter},
    utils::{
        self,
        syntect::syntect_load_defaults,
        types::{AsyncResult, StyledLine},
    },
};
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
use syntect::parsing::SyntaxReference;
use tokio::{
    fs,
    io::{self, AsyncBufReadExt},
    select,
    time::sleep,
};
use tokio_stream::StreamExt;
use tui::{backend::CrosstermBackend, Terminal};

const THEME: &str = "Solarized (dark)";

pub async fn run_host_session(opts: HostOptions) -> AsyncResult<()> {
    println!("> Hosting as `{}`", opts.username);

    let syntax = find_syntax_for_file(&opts.file)?;
    let text = fs::read_to_string(&opts.file).await?;

    let metadata = DacttyloMetadata {
        syntax_name: syntax.name.clone(),
        text: text.clone(),
    };

    let mut session = session::new().await?;
    println!("Local peer id: {:?}", session.peer_id);

    let (start_date, mut registered_users) =
        take_registrations(&mut session, metadata, &opts).await?;

    registered_users.remove(&session.peer_id.to_base58());
    let opponent_names: Vec<&str> =
        registered_users.values().map(|n| n.as_ref()).collect();

    let app = OnlineGame::new(
        session,
        Game::new(&text, &opponent_names, opts, THEME)?,
    );

    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let lines = highlight(&syntax.name, THEME, &lines)?;

    wake_up(Some(start_date)).await;

    let mut term = enter_tui_mode(std::io::stdout())?;
    let session_result =
        handle_events(&mut term, registered_users, app, &lines).await;

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

async fn take_registrations(
    session: &mut SessionHandle,
    metadata: DacttyloMetadata,
    opts: &HostOptions,
) -> AsyncResult<(DateTime<Utc>, HashMap<String, String>)> {
    session
        .client
        .host_session(&opts.username, serialize(&metadata)?)
        .await?;
    let mut registered_users: HashMap<String, String> = Default::default();
    registered_users.insert(session.peer_id.to_base58(), opts.username.clone());
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

pub fn find_syntax_for_file(
    file: &str,
) -> AsyncResult<&'static SyntaxReference> {
    let (syntax_set, _) = syntect_load_defaults();
    syntax_set
        .find_syntax_for_file(file)
        .map_err(|_| "error reading file")?
        .ok_or_else(|| "failed to find syntax".into())
}

pub fn highlight<'t>(
    name: &str,
    theme: &str,
    lines: &[&'t str],
) -> AsyncResult<Vec<StyledLine<'t>>> {
    let hl = SyntectHighlighter::new()
        .from_syntax(name)?
        .theme(get_theme(theme))
        .build()?;

    Ok(hl.highlight(lines))
}

pub async fn handle_events<O>(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    mut registered_users: HashMap<String, String>,
    mut app: OnlineGame<'_, O>,
    lines: &[StyledLine<'_>],
) -> AsyncResult<Option<SessionResult>> {
    loop {
        let event = select! {
            Some(event) = app.game.events.next() => event,
            Some(event) = app.session.events.next() => event.into()
        };

        let session_state =
            handle_event(event, &mut registered_users, &mut app).await?;

        if let SessionState::End(end) = session_state {
            // NOTE: last floodsub publish may not have been sent yet,
            // small delay to prevent the task from dropping too soon on process exit
            sleep(Duration::from_millis(10)).await;

            if let SessionEnd::Finished = &end {
                return Ok(Some(generate_session_result(app.game)));
            } else {
                return Ok(None);
            }
        }

        render(term, &app.game, lines)?;
    }
}

async fn handle_event<O>(
    event: AppEvent,
    registered_users: &mut HashMap<String, String>,
    app: &mut OnlineGame<'_, O>,
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

fn handle_session_event<O>(
    event: SessionEvent,
    registered_users: &mut HashMap<String, String>,
    game: &mut Game<O>,
) -> AsyncResult<SessionState> {
    let SessionEvent { peer_id, cmd } = event;

    if let SessionCommand::Push(payload) = cmd {
        let username = registered_users
            .get(&peer_id)
            .ok_or("session event origin user not found")?;

        match deserialize(&payload)? {
            DacttyloCommand::Input(ch) => {
                game.opponents.process_input(username, ch).ok();
            }
            DacttyloCommand::Forfeit => {
                game.opponents.remove(username);
                registered_users.remove(&peer_id);
            }
        }

        if game.main.is_done() && game.opponents.are_done() {
            return Ok(SessionState::End(SessionEnd::Finished));
        }
    }

    Ok(SessionState::Ongoing)
}

async fn handle_term<O>(
    term_event: crossterm::event::Event,
    app: &mut OnlineGame<'_, O>,
) -> AsyncResult<SessionState> {
    let client = &mut app.session.client;

    if let Event::Key(event) = term_event {
        let KeyEvent { code, .. } = event;
        let c = match code {
            KeyCode::Esc => {
                let serial = serialize(&DacttyloCommand::Forfeit)?;
                client.publish(SessionCommand::Push(serial)).await.unwrap();
                return Ok(SessionState::End(SessionEnd::Quit));
            }
            KeyCode::Char(c) => Some(c),
            KeyCode::Enter => Some('\n'),
            KeyCode::Tab => Some('\t'),
            _ => None,
        };

        if let Some(c) = c {
            let serial = serialize(&DacttyloCommand::Input(c))?;
            client.publish(SessionCommand::Push(serial)).await.unwrap();

            app.game.main.process_input(c);

            if app.game.main.is_done() && app.game.opponents.are_done() {
                return Ok(SessionState::End(SessionEnd::Finished));
            }
        }
    }

    Ok(SessionState::Ongoing)
}

fn generate_session_result<O>(game: Game<'_, O>) -> SessionResult {
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
