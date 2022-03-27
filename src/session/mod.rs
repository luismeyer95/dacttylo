pub mod client;
pub mod command;
pub mod data;
pub mod event;

pub use self::{
    client::SessionClient, command::SessionCommand, data::SessionData,
};

use crate::network::{P2PClient, P2PEvent};
use event::SessionEvent;
use futures::Stream;

pub fn new<T>(
    p2p_client: P2PClient,
    p2p_stream: T,
) -> (SessionClient, impl Stream<Item = SessionEvent>)
where
    T: Stream<Item = P2PEvent>,
{
    let session_stream = async_stream::stream! {
        for await net_event in p2p_stream {
            yield Into::<SessionEvent>::into(net_event);
        }
    };

    (SessionClient::new(p2p_client), session_stream)
}
