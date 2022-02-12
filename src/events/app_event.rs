use futures::Stream;
use tokio::sync::mpsc::{self, Sender};
use tokio_stream::wrappers::ReceiverStream;

use crate::{app::InputResult, session::event::SessionEvent};

#[derive(Debug)]
pub enum AppEvent {
    // external triggers
    Term(Result<crossterm::event::Event, std::io::Error>),
    Session(SessionEvent),

    // internal triggers
    Tick,
    WpmTick,
    GhostInput(InputResult),
}

pub fn stream() -> (Sender<AppEvent>, impl Stream<Item = AppEvent>) {
    let (tx, rx) = mpsc::channel::<AppEvent>(256);
    (tx, ReceiverStream::new(rx))
}
