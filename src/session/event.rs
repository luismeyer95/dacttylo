use super::SessionCommand;
use crate::{events::AppEvent, network::P2PEvent};
use bincode::deserialize;

#[derive(Debug, Clone)]
pub struct SessionEvent {
    pub peer_id: String,
    pub cmd: SessionCommand,
}

impl From<P2PEvent> for SessionEvent {
    fn from(e: P2PEvent) -> Self {
        let P2PEvent::TopicMessage { source, data, .. } = e;

        SessionEvent {
            peer_id: source.to_base58(),
            cmd: deserialize::<SessionCommand>(&data).unwrap(),
        }
    }
}

impl From<SessionEvent> for AppEvent {
    fn from(e: SessionEvent) -> Self {
        AppEvent::Session(e)
    }
}
