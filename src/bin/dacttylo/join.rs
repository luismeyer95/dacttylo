// use crate::{
//     common::*,
//     protocol::ProtocolCommand,
//     report::{display_session_report, Ranking, SessionResult},
// };
// use bincode::{deserialize, serialize};
// use chrono::{DateTime, Utc};
// use crossterm::event::{Event, KeyCode, KeyEvent};
// use dacttylo::{cli::HostOptions, utils::types::AsyncResult};
// use dacttylo::{
//     events::AppEvent,
//     game::{game::Game, online_game::OnlineGame},
//     session::{
//         self, event::SessionEvent, session_handle::SessionHandle,
//         SessionClient, SessionCommand,
//     },
//     utils::{
//         time::{datetime_in, wake_up},
//         tui::{enter_tui_mode, leave_tui_mode},
//     },
// };
// use std::{collections::HashMap, io::Stdout, iter, time::Duration};
// use tokio::{
//     fs,
//     io::{self, AsyncBufReadExt},
//     select,
// };
// use tokio_stream::StreamExt;
// use tui::{backend::CrosstermBackend, Terminal};

// pub async fn run_host_session(host_opts: HostOptions) -> AsyncResult<()> {
//     println!("> Hosting as `{}`", host_opts.username);

//     let text = fs::read_to_string(&host_opts.file).await?;

//     let mut session = session::new().await?;
//     println!("Local peer id: {:?}", session.peer_id);

//     let (start_date, registered_users) =
//         take_registrations(&mut session, &text, &host_opts).await?;

//     let opponent_names: Vec<&str> =
//         registered_users.iter().map(|(_, v)| v.as_ref()).collect();
//     let game =
//         OnlineGame::new(session, Game::new(&text, &opponent_names, host_opts)?);

//     wake_up(Some(start_date)).await;

//     let mut term = enter_tui_mode(std::io::stdout())?;
//     let session_result =
//         handle_events(&mut term, registered_users, game, &text).await;

//     let result = match session_result {
//         Ok(Some(session_result)) => {
//             display_session_report(&mut term, session_result).await
//         }
//         Ok(None) => Ok(()),
//         Err(e) => Err(e),
//     };

//     leave_tui_mode(term)?;
//     result
// }
