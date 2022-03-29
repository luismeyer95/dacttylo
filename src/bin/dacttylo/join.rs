use crate::{
    common::*,
    host::{handle_events, highlight},
    join,
    protocol::{DacttyloCommand, DacttyloMetadata},
    report::{display_session_report, Ranking, SessionResult},
};
use bincode::{deserialize, serialize};
use chrono::{DateTime, Utc};
use crossterm::event::{Event, KeyCode, KeyEvent};
use dacttylo::{
    cli::{HostOptions, JoinOptions},
    session::SessionData,
    utils::types::AsyncResult,
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
use tokio::{
    fs,
    io::{self, AsyncBufReadExt},
    select,
};
use tokio_stream::StreamExt;
use tui::{backend::CrosstermBackend, Terminal};

const THEME: &str = "Solarized (dark)";

async fn register(
    session: &mut SessionHandle,
    opts: &JoinOptions,
) -> AsyncResult<(DacttyloMetadata, DateTime<Utc>, HashMap<String, String>)> {
    let client = &mut session.client;

    let SessionData {
        session_id,
        metadata,
    } = client.await_session_for_host(&opts.host).await;
    let metadata = deserialize(&metadata)?;

    println!("Session found!");
    client.join_session(session_id.clone()).await?;
    println!("Joined session `{}`", session_id.clone());

    client
        .publish(SessionCommand::Register {
            user: opts.username.clone(),
        })
        .await?;
    println!("Submitted registration...");

    loop {
        select! {
            // handle session events
            event = session.events.next() => {
                let event = event.ok_or("event stream closed unexpectedly")?;
                let SessionEvent {
                    peer_id, cmd
                } = event.into();

                if let SessionCommand::LockSession { registered_users, session_start } = cmd {
                    let session_start: DateTime<Utc> = session_start.parse().map_err(|_| "invalid date time for session start")?;
                    return Ok((metadata, session_start, registered_users));
                }
            }
        };
    }
}

pub async fn run_join_session(join_opts: JoinOptions) -> AsyncResult<()> {
    println!("> Joining as `{}`", join_opts.username);

    let mut session = session::new().await?;
    println!("Local peer id: {:?}", session.peer_id);

    let (metadata, start_date, mut registered_users) =
        register(&mut session, &join_opts).await?;

    let delay = start_date.signed_duration_since(Utc::now());
    let delay = chrono::Duration::to_std(&delay).unwrap();
    println!("Session locked! Starting in {:?}...", delay);

    registered_users.remove(&session.peer_id.to_base58());
    let opponent_names: Vec<&str> =
        registered_users.values().map(|n| n.as_ref()).collect();

    let game = OnlineGame::new(
        session,
        Game::new(&metadata.text, &opponent_names, join_opts, THEME)?,
    );

    let lines: Vec<&str> = metadata.text.split_inclusive('\n').collect();
    let lines = highlight(&metadata.syntax_name, THEME, &lines)?;

    wake_up(Some(start_date)).await;

    let mut term = enter_tui_mode(std::io::stdout())?;
    let session_result =
        handle_events(&mut term, registered_users, game, &lines).await;

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
