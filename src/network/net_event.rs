use libp2p::{floodsub::Topic, PeerId};

#[derive(Clone, Debug)]
pub enum P2PEvent {
    TopicMessage {
        source: PeerId,
        topics: Vec<Topic>,
        data: Vec<u8>,
    },
}
