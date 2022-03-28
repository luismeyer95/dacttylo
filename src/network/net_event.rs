use libp2p::{floodsub::Topic, PeerId};

use crate::events::AppEvent;

#[derive(Clone, Debug)]
pub enum P2PEvent {
    TopicMessage {
        source: PeerId,
        topics: Vec<Topic>,
        data: Vec<u8>,
    },
}

impl From<P2PEvent> for AppEvent {
    fn from(e: P2PEvent) -> Self {
        AppEvent::Session(e.into())
    }
}
