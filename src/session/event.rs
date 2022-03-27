use libp2p::{floodsub::Topic, PeerId};

use crate::{events::AppEvent, network::P2PEvent};

#[derive(Debug, Clone)]
pub enum SessionEvent {
    TopicMessage {
        source: PeerId,
        topics: Vec<Topic>,
        data: Vec<u8>,
    },
}

impl From<P2PEvent> for SessionEvent {
    fn from(e: P2PEvent) -> Self {
        let P2PEvent::TopicMessage {
            source,
            topics,
            data,
        } = e;
        SessionEvent::TopicMessage {
            source,
            topics,
            data,
        }
    }
}

impl From<SessionEvent> for AppEvent {
    fn from(e: SessionEvent) -> Self {
        AppEvent::Session(e)
    }
}
