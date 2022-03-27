pub mod event_loop;
pub mod net_command;
pub mod net_event;
pub mod p2p_client;

pub use event_loop::EventLoop;
pub use net_command::NetCommand;
pub use net_event::P2PEvent;
pub use p2p_client::P2PClient;

use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed, upgrade},
    floodsub::Floodsub,
    identity,
    kad::{store::MemoryStore, Kademlia},
    mdns::Mdns,
    mplex,
    noise::{self, AuthenticKeypair, X25519Spec},
    swarm::SwarmBuilder,
    tcp::TokioTcpConfig,
    PeerId, Swarm, Transport,
};
use std::error::Error;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, Stream};

use self::event_loop::Behaviour;

type AsyncResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub fn generate_noise_keys(
    id_keys: &identity::Keypair,
) -> AuthenticKeypair<X25519Spec> {
    noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(id_keys)
        .expect("Signing libp2p-noise static DH keypair failed.")
}

pub fn generate_transport(
    noise_keys: AuthenticKeypair<X25519Spec>,
) -> Boxed<(PeerId, StreamMuxerBox)> {
    TokioTcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed()
}

pub async fn generate_swarm(
    peer_id: PeerId,
    transport: Boxed<(PeerId, StreamMuxerBox)>,
) -> AsyncResult<Swarm<Behaviour>> {
    let mdns = Mdns::new(Default::default()).await?;

    let kademlia = {
        let store = MemoryStore::new(peer_id);
        Kademlia::new(peer_id, store)
    };

    let floodsub = Floodsub::new(peer_id);

    let behaviour = event_loop::Behaviour {
        mdns,
        kademlia,
        floodsub,
    };

    Ok(SwarmBuilder::new(transport, behaviour, peer_id)
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build())
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
) -> AsyncResult<(P2PClient, impl Stream<Item = P2PEvent> + 'static, EventLoop)>
{
    let peer_id = PeerId::from(id_keys.public());

    // Create a keypair for authenticated encryption of the transport
    let noise_keys = generate_noise_keys(&id_keys);

    // Create a tokio-based TCP transport use noise for authenticated
    // encryption and Mplex for multiplexing of substreams on a TCP stream
    let transport = generate_transport(noise_keys);

    // Create a Swarm to manage peers and events
    let mut swarm = generate_swarm(peer_id, transport).await?;

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let (command_sender, command_receiver) = mpsc::channel(256);
    let (event_sender, event_receiver) = mpsc::channel(256);

    Ok((
        P2PClient::new(command_sender),
        ReceiverStream::new(event_receiver),
        EventLoop::new(
            swarm,
            ReceiverStream::new(command_receiver),
            event_sender,
        ),
    ))
}
