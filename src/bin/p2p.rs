#![allow(unused)]

use chrono::{DateTime, Utc};
use libp2p::{
    core::upgrade,
    identity,
    kad::{record::Key, Quorum},
    mplex, noise,
    swarm::{SwarmBuilder, SwarmEvent},
    tcp::TokioTcpConfig,
    PeerId, Swarm, Transport,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

use dacttylo::{
    self,
    cli::*,
    network::{self, NetEvent},
    session::{SessionCommand, SessionData},
    utils::time::*,
};
use tokio::io::{self, AsyncBufReadExt};
use tokio_stream::{Stream, StreamExt};

type AsyncResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[tokio::main]
async fn main() {
    // env_logger::init();

    let cli = dacttylo::cli::parse();

    if let Err(e) = match cli.command.clone() {
        Commands::Host { user, file } => handle_host(user, file).await,
        Commands::Join { user, host } => handle_join(user, host).await,
    } {
        eprintln!("Error: {}", e);
    }
}

async fn handle_host(user: String, file: String) -> AsyncResult<()> {
    println!("'Host' was used, name is: {:?}", user);

    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());

    println!("Local peer id: {:?}", peer_id);

    let (mut p2p_client, mut event_stream, task) = network::new(id_keys.clone()).await?;
    let mut client = dacttylo::session::SessionClient::new(p2p_client);

    tokio::spawn(task.run());
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    let session_id = "abcd";
    let text = std::fs::read_to_string(&file)?;

    client
        .host_session(
            &user,
            SessionData {
                session_id: session_id.into(),
                metadata: text.as_str().into(),
            },
        )
        .await
        .unwrap_err();

    enum State {
        TakingRegistrations,
        AwaitingSessionStart,
        SessionStarted,
    }

    let mut state: State = State::TakingRegistrations;
    let mut registered_users: HashMap<String, String> = Default::default();

    // Insert the host in the id/username map
    registered_users.insert(peer_id.to_base58(), user.clone());

    let timer = wake_up(None);
    let mut timer_active = false;
    tokio::pin!(timer);

    loop {
        tokio::select! {
            // await timer if active
            _ = &mut timer, if timer_active => {
                state = State::SessionStarted;
                timer_active = false;
                println!("*** SESSION START ***\n{}", text);
            }

            // handle user input
            line = stdin.next_line() => {
                let _line = line?.expect("Standard input was closed");

                match state {
                    // lock registrations when host presses enter
                    State::TakingRegistrations => {
                        let date = datetime_in(Duration::from_secs(3)).unwrap();
                        let lock_cmd = SessionCommand::LockSession { registered_users: registered_users.clone(), session_start: date.to_string()  };

                        println!("Locking session...");
                        client.publish(lock_cmd).await?;
                        println!("Session locked, starting soon :)");

                        state = State::AwaitingSessionStart;
                        timer.set(wake_up(Some(date)));
                        timer_active = true;
                    },

                    // awaiting session start, do not process anything
                    State::AwaitingSessionStart => {}

                    // publish user payload
                    State::SessionStarted => {
                        client.publish(SessionCommand::Push(_line.into())).await?;
                    }

                }

            }

            // handle session events
            event = event_stream.next() => {

                match event {
                    Some(e) => {
                        let NetEvent::TopicMessage {
                            source, topics, data
                        } = e;
                        let (peer_id, cmd) = (source, bincode::deserialize::<SessionCommand>(&data)?);

                        match &state {
                            // process registrations if user hasn't locked session
                            State::TakingRegistrations => {
                                if let SessionCommand::Register { user } = cmd {
                                    println!("Registering user `{}`", user);
                                    registered_users.insert(peer_id.to_base58(), user);
                                };
                            },

                            // awaiting session start, do not process anything
                            State::AwaitingSessionStart => {}

                            // take in payloads and process them
                            State::SessionStarted => {
                                if let SessionCommand::Push(payload) = cmd {
                                    let username = registered_users.get(&peer_id.to_base58()).expect("Session event origin user not found");
                                    println!("{}: {}", username, String::from_utf8_lossy(&payload));
                                }
                            }

                        }
                    }
                    _ => {
                        eprintln!("Event stream was closed");
                    },
                }
            }
        };
    }
}

async fn handle_join(user: String, host: String) -> AsyncResult<()> {
    println!(
        "'Join' was used, name is: {:?}, joining host {:?}",
        user, host
    );

    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());

    println!("Local peer id: {:?}", peer_id);

    let (mut p2p_client, mut event_stream, task) = network::new(id_keys).await?;
    let mut client = dacttylo::session::SessionClient::new(p2p_client);

    tokio::spawn(task.run());
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    let SessionData {
        session_id,
        metadata,
    } = loop {
        println!("Searching session...");
        tokio::time::sleep(Duration::from_millis(300)).await;
        if let Ok(data) = client.get_hosted_session_data(&host).await {
            break data;
        }
    };
    println!("Session found!");
    let text = String::from_utf8(metadata)?;

    client.join_session(session_id.clone()).await?;
    println!("Joined session `{}`", session_id.clone());

    client.publish(SessionCommand::Register { user }).await?;
    println!("Submitted registration...");

    let mut timer_active = false;
    let timer = wake_up(None);
    tokio::pin!(timer);

    enum State {
        AwaitingSessionStart,
        SessionStarted,
    }

    let mut state = State::AwaitingSessionStart;
    let mut session_users: HashMap<String, String> = Default::default();

    loop {
        tokio::select! {
             // await timer if active
             _ = &mut timer, if timer_active => {
                state = State::SessionStarted;
                timer_active = false;
                println!("*** SESSION START ***\n{}", text);
            }

            line = stdin.next_line() => {
                let _line = line?.expect("Standard input was closed");

                match state {
                    // no user input until session start
                    State::AwaitingSessionStart => {
                        println!("Please wait for the session to start.");
                    },
                    // publish user payload
                    State::SessionStarted => {
                        client.publish(SessionCommand::Push(_line.into())).await?;
                    }

                }
            }

            // handle session events
            event = event_stream.next() => {

                match event {
                    Some(e) => {
                        let NetEvent::TopicMessage {
                            source, topics, data
                        } = e;
                        let (peer_id, cmd) = (source, bincode::deserialize::<SessionCommand>(&data)?);

                        match state {

                            // awaiting session start, do not process anything
                            State::AwaitingSessionStart => {
                                if let SessionCommand::LockSession { registered_users, session_start } = cmd {
                                    session_users = registered_users;

                                    let session_start: DateTime<Utc> = session_start.parse().expect("Invalid date time for session start");

                                    timer.set(wake_up(Some(session_start)));
                                    timer_active = true;

                                    let delay = session_start.signed_duration_since(Utc::now());
                                    let delay = chrono::Duration::to_std(&delay).unwrap();
                                    println!("Session locked! Starting in {:?}...", delay);
                                }
                            }

                            // take in remote user payloads and process them
                            State::SessionStarted => {
                                if let SessionCommand::Push(payload) = cmd {
                                    let username = session_users.get(&peer_id.to_base58()).expect("Session event origin user not found");
                                    println!("{}: {}", username, String::from_utf8_lossy(&payload));
                                }
                            }

                        }
                    }
                    _ => {
                        eprintln!("Event stream was closed");
                    },
                }
            }
        };
    }
}
