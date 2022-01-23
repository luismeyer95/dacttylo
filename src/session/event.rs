use futures::Stream;
use libp2p::{floodsub::Topic, PeerId};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::{events::AppEvent, network::NetEvent};

#[derive(Debug, Clone)]
pub enum SessionEvent {
    TopicMessage {
        source: PeerId,
        topics: Vec<Topic>,
        data: Vec<u8>,
    },
}

impl From<NetEvent> for SessionEvent {
    fn from(e: NetEvent) -> Self {
        let NetEvent::TopicMessage {
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
