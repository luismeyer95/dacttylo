use super::SessionClient;
use crate::network::P2PEvent;
use futures::Stream;
use libp2p::PeerId;
use tokio_stream::wrappers::ReceiverStream;

pub struct SessionHandle {
    pub client: SessionClient,
    pub events: ReceiverStream<P2PEvent>,
    pub peer_id: PeerId,
}
