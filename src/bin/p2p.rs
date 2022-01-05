#![allow(unused)]

use chrono::{DateTime, Utc};
use clap::{AppSettings, Parser, Subcommand};
use libp2p::{
    core::upgrade,
    identity,
    kad::{record::Key, Quorum},
    mplex, noise,
    swarm::{dial_opts::PeerCondition, PollParameters, SwarmBuilder, SwarmEvent},
    tcp::TokioTcpConfig,
    PeerId, Swarm, Transport,
};
use network::{NetEvent, SessionCommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

use tokio::io::{self, AsyncBufReadExt};
use tokio_stream::{Stream, StreamExt};

type AsyncResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Session handle and context needed to join a session.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct SessionData {
    /// Used as the floodsub topic.
    session_id: String,

    /// Session specific data.
    metadata: Vec<u8>,
}

#[derive(Parser)]
#[clap(author, version, about)]
#[clap(global_setting(AppSettings::PropagateVersion))]
#[clap(global_setting(AppSettings::UseLongFormatForHelpSubcommand))]
#[clap(setting(AppSettings::SubcommandRequiredElseHelp))]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone, Debug)]
enum Commands {
    /// Host a game
    Host {
        /// Your username
        #[clap(short, long)]
        user: String,

        /// Path of the file to race on
        #[clap(short, long)]
        file: String,
    },
    /// Join a game
    Join {
        /// The host to join
        host: String,

        /// Your username
        #[clap(short, long)]
        user: String,
    },
}

fn datetime_in(delay: Duration) -> Option<DateTime<Utc>> {
    let chrono_delay = chrono::Duration::from_std(delay).ok()?;
    let future_date = Utc::now().checked_add_signed(chrono_delay)?;
    Some(future_date)
}

async fn wake_up(at: Option<DateTime<Utc>>) -> Option<()> {
    let at = at?;

    let delay = at.signed_duration_since(Utc::now());
    let delay = chrono::Duration::to_std(&delay).unwrap();
    tokio::time::sleep(delay).await;

    Some(())
}

async fn handle_host(user: String, file: String) -> AsyncResult<()> {
    println!("'Host' was used, name is: {:?}", user);

    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());

    println!("Local peer id: {:?}", peer_id);

    let (mut client, mut event_stream, task) = network::new(id_keys.clone()).await.unwrap();

    tokio::spawn(task.run());
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    let session_id = "abcd";
    let text = "The quick brown fox jumped over the lazy dog";

    client
        .host_session(
            &user,
            SessionData {
                session_id: session_id.into(),
                metadata: text.into(),
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

    let (mut client, mut event_stream, task) = network::new(id_keys).await.unwrap();
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

/// The `tokio::main` attribute sets up a tokio runtime.
#[tokio::main]
async fn main() {
    // env_logger::init();

    let cli = Cli::parse();
    if let Err(e) = match cli.command.clone() {
        Commands::Host { user, file } => handle_host(user, file).await,
        Commands::Join { user, host } => handle_join(user, host).await,
    } {
        eprintln!("Error: {}", e);
    }
}

mod network {

    use libp2p::{
        floodsub::{Floodsub, FloodsubEvent, FloodsubMessage, Topic},
        kad::{
            store::MemoryStore, GetRecordResult, Kademlia, KademliaEvent, PeerRecord,
            PutRecordResult, QueryId, QueryResult, Record,
        },
        mdns::{Mdns, MdnsEvent},
        NetworkBehaviour,
    };
    use std::collections::HashMap;
    use tokio::sync::{mpsc, oneshot};
    use tokio_stream::wrappers::ReceiverStream;

    use super::*;

    #[derive(Clone)]
    pub struct P2PClient {
        sender: mpsc::Sender<NetCommand>,
    }

    impl P2PClient {
        pub async fn put_record(
            &self,
            key: Key,
            value: impl Into<Vec<u8>>,
        ) -> Result<PutRecordResult, Box<dyn Error>> {
            let (tx, rx) = oneshot::channel();
            self.sender
                .send(network::NetCommand::PutRecord {
                    key: key.clone(),
                    value: value.into(),
                    sender: tx,
                })
                .await?;

            Ok(rx.await?)
        }

        pub async fn get_record(&self, key: Key) -> Result<GetRecordResult, Box<dyn Error>> {
            let (tx, rx) = oneshot::channel();
            self.sender
                .send(network::NetCommand::GetRecord { key, sender: tx })
                .await?;

            Ok(rx.await?)
        }

        pub async fn remove_record(&self, key: Key) -> Result<(), Box<dyn Error>> {
            let (tx, rx) = oneshot::channel();
            self.sender
                .send(network::NetCommand::RemoveRecord { key, sender: tx })
                .await?;

            Ok(rx.await?)
        }

        pub async fn subscribe(&self, topic: Topic) -> Result<bool, Box<dyn Error>> {
            let (tx, rx) = oneshot::channel();
            self.sender
                .send(network::NetCommand::Sub { topic, sender: tx })
                .await?;

            Ok(rx.await?)
        }

        pub async fn unsubscribe(&self, topic: Topic) -> Result<bool, Box<dyn Error>> {
            let (tx, rx) = oneshot::channel();
            self.sender
                .send(network::NetCommand::Unsub { topic, sender: tx })
                .await?;

            Ok(rx.await?)
        }

        pub async fn publish(
            &self,
            topic: Topic,
            payload: impl Into<Vec<u8>>,
        ) -> Result<(), Box<dyn Error>> {
            let (tx, rx) = oneshot::channel();
            self.sender
                .send(network::NetCommand::Publish {
                    topic,
                    payload: payload.into(),
                    sender: tx,
                })
                .await?;

            Ok(rx.await?)
        }
    }

    #[derive(Clone)]
    pub struct Client {
        p2p_client: P2PClient,
        current_session_id: Option<String>,
    }

    impl Client {
        fn get_session(&self) -> Result<&str, &'static str> {
            self.current_session_id
                .as_ref()
                .map(|s| s.as_str())
                .ok_or("Session not found")
        }

        pub async fn get_hosted_session_data(&mut self, host: &str) -> AsyncResult<SessionData> {
            let key = Key::new(&host);
            let err_str = format!("Could not find record `{:?}`", key);

            let mut result = self
                .p2p_client
                .get_record(key.clone())
                .await
                .expect("P2P client channel failure")
                .map_err(|_| err_str.clone())?;

            let PeerRecord {
                record: Record { value, .. },
                ..
            } = result.records.pop().ok_or(err_str)?;

            Ok(bincode::deserialize(&value)?)
        }

        pub async fn host_session(
            &mut self,
            host: &str,
            session_data: SessionData,
        ) -> AsyncResult<()> {
            let s_id = session_data.session_id.clone();
            self.join_session(s_id.into()).await?;

            let key = Key::new(&host);
            let value = bincode::serialize(&session_data)?;

            let result = self
                .p2p_client
                .put_record(key.clone(), value)
                .await
                .expect("P2P client channel failure");

            match result {
                Ok(_) => Ok(()),
                Err(_) => Err(format!("Could not put record `{:?}`", key).into()),
            }
        }

        pub async fn stop_hosting_session(&mut self, host: &str) -> AsyncResult<()> {
            let err_p2p = "P2P client channel failure";

            self.leave_session().await?;

            self.p2p_client
                .remove_record(Key::new(&host))
                .await
                .expect(err_p2p);

            self.current_session_id = None;

            Ok(())
        }

        pub async fn join_session(&mut self, session_id: String) -> AsyncResult<bool> {
            let result = self
                .p2p_client
                .subscribe(Topic::new(session_id.clone()))
                .await
                .expect("P2P client channel failure");

            self.current_session_id = Some(session_id);

            Ok(result)
        }

        pub async fn leave_session(&mut self) -> AsyncResult<bool> {
            let current_session_id = self.get_session()?;

            let result = self
                .p2p_client
                .unsubscribe(Topic::new(current_session_id))
                .await
                .expect("P2P client channel failure");

            self.current_session_id = None;

            Ok(result)
        }

        pub async fn publish(&mut self, session_cmd: SessionCommand) -> AsyncResult<()> {
            let current_session_id = self.get_session()?;
            let payload = bincode::serialize(&session_cmd)?;

            Ok(self
                .p2p_client
                .publish(Topic::new(current_session_id), payload)
                .await
                .expect("P2P client channel failure"))
        }
    }

    #[derive(NetworkBehaviour)]
    #[behaviour(out_event = "ComposedEvent")]
    pub struct MyBehaviour {
        pub floodsub: Floodsub,
        pub kademlia: Kademlia<MemoryStore>,
        pub mdns: Mdns,
    }

    #[derive(Debug)]
    pub enum ComposedEvent {
        Floodsub(FloodsubEvent),
        Kademlia(KademliaEvent),
        Mdns(MdnsEvent),
    }

    impl From<KademliaEvent> for ComposedEvent {
        fn from(event: KademliaEvent) -> Self {
            ComposedEvent::Kademlia(event)
        }
    }

    impl From<MdnsEvent> for ComposedEvent {
        fn from(event: MdnsEvent) -> Self {
            ComposedEvent::Mdns(event)
        }
    }

    impl From<FloodsubEvent> for ComposedEvent {
        fn from(event: FloodsubEvent) -> Self {
            ComposedEvent::Floodsub(event)
        }
    }

    /// Creates the network components, namely:
    ///
    /// - The network client to interact with the network layer from anywhere
    ///   within your application.
    ///
    /// - The network event stream, e.g. for incoming requests.
    ///
    /// - The network task driving the network itself.
    pub async fn new(
        id_keys: identity::Keypair,
    ) -> AsyncResult<(Client, impl Stream<Item = NetEvent>, EventLoop)> {
        let peer_id = PeerId::from(id_keys.public());

        // Create a keypair for authenticated encryption of the transport.
        let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&id_keys)
            .expect("Signing libp2p-noise static DH keypair failed.");

        // Create a tokio-based TCP transport use noise for authenticated
        // encryption and Mplex for multiplexing of substreams on a TCP stream.
        let transport = TokioTcpConfig::new()
            .nodelay(true)
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(mplex::MplexConfig::new())
            .boxed();

        // Create a Swarm to manage peers and events.
        let mut swarm = {
            let mdns = Mdns::new(Default::default()).await?;

            let kademlia = {
                let store = MemoryStore::new(peer_id);
                Kademlia::new(peer_id, store)
            };

            let floodsub = Floodsub::new(peer_id.clone());

            let behaviour = network::MyBehaviour {
                mdns,
                kademlia,
                floodsub,
            };
            // behaviour.floodsub.subscribe(floodsub_topic.clone());
            SwarmBuilder::new(transport, behaviour, peer_id)
                // We want the connection background tasks to be spawned
                // onto the tokio runtime.
                .executor(Box::new(|fut| {
                    tokio::spawn(fut);
                }))
                .build()
        };

        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        let (command_sender, command_receiver) = mpsc::channel(256);
        let (event_sender, event_receiver) = mpsc::channel(256);

        Ok((
            Client {
                p2p_client: P2PClient {
                    sender: command_sender,
                },
                current_session_id: None,
            },
            ReceiverStream::new(event_receiver),
            EventLoop::new(swarm, ReceiverStream::new(command_receiver), event_sender),
        ))
    }

    /// Communication protocol for joining, starting,
    /// and stopping sessions.
    #[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
    pub enum SessionCommand {
        /// Announce participation to the session.
        /// The host will keep track of registered users
        /// by storing a PeerId => Username map which will
        /// be published on the topic prior to locking the
        /// session.
        Register { user: String },

        /// Command issued by the session host to communicate
        /// that registrations are closed and the session is
        /// scheduled to start at `session_start`
        LockSession {
            /// Map from peer_id to username
            /// (peer_id is not serializable, default to using a string)
            registered_users: HashMap<String, String>,

            /// Datetime of the scheduled session
            session_start: String,
        },

        /// Application specific push payload, what is sent
        /// is only relevant to the API user
        Push(Vec<u8>),

        /// Command issued by the session host to communicate
        /// the end of the session
        EndSession,
    }

    #[derive(Debug)]
    enum NetCommand {
        PutRecord {
            key: Key,
            value: Vec<u8>,
            sender: oneshot::Sender<PutRecordResult>,
        },

        GetRecord {
            key: Key,
            sender: oneshot::Sender<GetRecordResult>,
        },

        RemoveRecord {
            key: Key,
            sender: oneshot::Sender<()>,
        },

        Sub {
            topic: Topic,
            sender: oneshot::Sender<bool>,
        },

        Unsub {
            topic: Topic,
            sender: oneshot::Sender<bool>,
        },

        Publish {
            topic: Topic,
            payload: Vec<u8>,
            sender: oneshot::Sender<()>,
        },
    }

    #[derive(Clone, Debug)]
    pub enum NetEvent {
        TopicMessage {
            source: PeerId,
            topics: Vec<Topic>,
            data: Vec<u8>,
        },
    }

    pub struct EventLoop {
        swarm: Swarm<MyBehaviour>,
        command_receiver: ReceiverStream<NetCommand>,
        event_sender: mpsc::Sender<NetEvent>,

        pending_get_record: HashMap<QueryId, oneshot::Sender<GetRecordResult>>,
        pending_put_record: HashMap<QueryId, oneshot::Sender<PutRecordResult>>,
    }

    impl EventLoop {
        fn new(
            swarm: Swarm<MyBehaviour>,
            command_receiver: ReceiverStream<NetCommand>,
            event_sender: mpsc::Sender<NetEvent>,
        ) -> Self {
            Self {
                swarm,
                command_receiver,
                event_sender,
                pending_get_record: Default::default(),
                pending_put_record: Default::default(),
            }
        }

        pub async fn run(mut self) {
            loop {
                tokio::select! {
                    event = self.swarm.next() => {
                        let event = event.expect("Swarm stream ended unexpectedly");
                        self.handle_event(event).await;
                    },
                    command = self.command_receiver.next() => match command {
                        Some(c) => self.handle_command(c).await,
                        // Command channel closed, thus shutting down the network event loop.
                        None=>  return,
                    },
                }
            }
        }

        async fn handle_command(&mut self, command: NetCommand) {
            match command {
                NetCommand::GetRecord { key, sender } => {
                    let query_id = self
                        .swarm
                        .behaviour_mut()
                        .kademlia
                        .get_record(&key, Quorum::One);

                    self.pending_get_record.insert(query_id, sender);
                }

                NetCommand::PutRecord { key, value, sender } => {
                    let record = Record {
                        key: key.clone(),
                        value,
                        publisher: None,
                        expires: None,
                    };
                    let query_id = self
                        .swarm
                        .behaviour_mut()
                        .kademlia
                        .put_record(record, Quorum::One)
                        .expect("Failed to store record locally");

                    self.pending_put_record.insert(query_id, sender);
                }

                NetCommand::RemoveRecord { key, sender } => {
                    self.swarm.behaviour_mut().kademlia.remove_record(&key);
                    sender
                        .send(())
                        .expect("Unexpected closed P2P client receiver");
                }

                NetCommand::Sub { topic, sender } => {
                    let result = self.swarm.behaviour_mut().floodsub.subscribe(topic);
                    sender
                        .send(result)
                        .expect("Unexpected closed P2P client receiver");
                }

                NetCommand::Unsub { topic, sender } => {
                    let result = self.swarm.behaviour_mut().floodsub.unsubscribe(topic);
                    sender
                        .send(result)
                        .expect("Unexpected closed P2P client receiver");
                }

                NetCommand::Publish {
                    topic,
                    payload,
                    sender,
                } => {
                    self.swarm.behaviour_mut().floodsub.publish(topic, payload);
                    sender
                        .send(())
                        .expect("Unexpected closed P2P client receiver");
                } // _ => {}
            }
        }

        async fn handle_event(
            &mut self,
            event: SwarmEvent<ComposedEvent, impl Error + Send + Sync + 'static>,
        ) {
            match event {
                SwarmEvent::Behaviour(event) => match event {
                    ComposedEvent::Floodsub(e) => self.handle_floodsub_event(e).await,
                    ComposedEvent::Kademlia(e) => self.handle_kademlia_event(e).await,
                    ComposedEvent::Mdns(e) => self.handle_mdns_event(e).await,
                },

                // SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                //     println!("Connection established with {:?}", peer_id);
                // }

                // SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                //     println!("Connection closed with {:?} because {:?}", peer_id, cause);
                // }
                _ => {}
            }
        }

        async fn handle_floodsub_event(&mut self, event: FloodsubEvent) {
            match event {
                FloodsubEvent::Message(FloodsubMessage {
                    source,
                    topics,
                    data,
                    ..
                }) => self
                    .event_sender
                    .send(NetEvent::TopicMessage {
                        source,
                        topics,
                        data,
                    })
                    .await
                    .expect("Unexpected closed P2P client receiver"),

                // FloodsubEvent::Subscribed { peer_id, topic } => {
                //     println!("{:?} subscribed to topic {:?}", peer_id, topic);
                // }
                _ => {}
            }
        }

        async fn handle_mdns_event(&mut self, event: MdnsEvent) {
            let behaviour = self.swarm.behaviour_mut();
            match event {
                MdnsEvent::Discovered(list) => {
                    for (peer, multiaddr) in list {
                        behaviour.floodsub.add_node_to_partial_view(peer);
                        behaviour.kademlia.add_address(&peer, multiaddr);
                        // println!("Discovered {:?}", peer);
                    }
                }
                MdnsEvent::Expired(list) => {
                    for (peer, _multiaddr) in list {
                        if !behaviour.mdns.has_node(&peer) {
                            behaviour.floodsub.remove_node_from_partial_view(&peer);
                            // self.kademlia.remove_address(&peer, &multiaddr);
                        }
                    }
                }
            }
        }

        async fn handle_kademlia_event(&mut self, event: KademliaEvent) {
            match event {
                KademliaEvent::OutboundQueryCompleted { result, id, .. } => match result {
                    QueryResult::PutRecord(result) => {
                        let sender = self
                            .pending_put_record
                            .remove(&id)
                            .expect("Failed to retrieve pending put record operation");
                        sender
                            .send(result)
                            .expect("Unexpected closed P2P client receiver");
                    }

                    QueryResult::GetRecord(result) => {
                        let sender = self
                            .pending_get_record
                            .remove(&id)
                            .expect("Failed to retrieve pending get record operation");
                        sender
                            .send(result)
                            .expect("Unexpected closed P2P client receiver");
                    }

                    _ => {}
                },
                _ => {}
            }
        }
    }
}
