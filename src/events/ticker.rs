use crate::events::AppEvent;
use std::{
    ops::Deref,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::mpsc::{self, Sender};

#[allow(unused_imports)]
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};

pub fn new() -> (TickerClient, impl Stream<Item = TickEvent>) {
    let (tx, rx) = mpsc::channel::<TickEvent>(256);
    (TickerClient { tx }, ReceiverStream::new(rx))
}

#[derive(Debug, Clone, Copy)]
pub struct TickEvent;

pub struct TickerClient {
    tx: Sender<TickEvent>,
}

impl Deref for TickerClient {
    type Target = Sender<TickEvent>;

    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}

impl From<TickEvent> for AppEvent {
    fn from(_: TickEvent) -> Self {
        AppEvent::Tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn send_a_single_tick() {
        let (client, mut stream) = new();

        tokio::spawn(async move {
            client.send(TickEvent).await.unwrap();
        });

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(500)) => panic!("Timeout"),
            tick = stream.next() => assert!(tick.is_some())
        }
    }
}
