pub mod client;
pub mod command;
pub mod data;
pub mod event;
pub mod session_handle;

use self::session_handle::SessionHandle;
pub use self::{
    client::SessionClient, command::SessionCommand, data::SessionData,
};
use crate::{
    network::{self, P2PClient, P2PEvent},
    utils::types::AsyncResult,
};
use event::SessionEvent;
use futures::Stream;
use libp2p::{identity, PeerId};

pub async fn new() -> AsyncResult<SessionHandle> {
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    // println!("Local peer id: {:?}", peer_id);
    let (client, events, task) = network::new(id_keys.clone()).await?;

    tokio::spawn(task.run());

    Ok(SessionHandle {
        client: SessionClient::new(client),
        events,
        peer_id,
    })
}
