use bincode;
use message_io::events::EventReceiver;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeHandler, NodeTask, StoredNodeEvent};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;

use super::errors::NetMessageError;

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct Identifier {
    pub name: String,
    pub id: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub from: Identifier,
    pub buffer: Vec<u8>,
}

pub struct RemoteEvents {
    addr: String,
    handler: NodeHandler<()>,
    endpoint: Endpoint,
    receiver: EventReceiver<StoredNodeEvent<()>>,
    task: NodeTask,

    id: Identifier,
    subscriptions: Vec<String>,
}

impl RemoteEvents {
    pub fn new(name: &str) -> RemoteEvents {
        let id = Identifier {
            name: name.to_string(),
            id: rand::random::<u16>(),
        };
        let subscriptions = Vec::<String>::default();
        let (handler, listener) = node::split::<()>();
        let addr = "239.255.0.1:3010".to_string();
        let (endpoint, _) = handler.network().connect(Transport::Udp, &addr).unwrap();
        let (task, receiver) = listener.enqueue();
        Self {
            id,
            subscriptions,
            handler,
            endpoint,
            receiver,
            task,
            addr,
        }
    }

    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        let maybe_event = self.receiver.receive_timeout(Duration::from_millis(1000));
        let node_event = maybe_event.ok_or(NetMessageError::ConnectionTimeout)?;
        if let NetEvent::Connected(_, _always_true_for_udp) = node_event.network().borrow() {
            println!("Notifying on the network");
            self.handler
                .network()
                .listen(Transport::Udp, &self.addr)
                .unwrap();
            Ok(())
        } else {
            Err(NetMessageError::UnexpectedEvent.into())
        }
    }

    pub fn subscribe(&mut self, name: &str) {
        self.subscriptions.push(name.to_string());
    }

    pub fn poll(&mut self) -> Vec<Message> {
        let mut buffered: Vec<Message> = vec![];
        while let Some(event) = self.receiver.try_receive() {
            match event.network().borrow() {
                NetEvent::Message(_, data) => {
                    let m: Message = bincode::deserialize(data).unwrap();
                    if m.from != self.id && self.subscriptions.contains(&m.from.name) {
                        buffered.push(m);
                    }
                }
                NetEvent::Accepted(_, _) => unreachable!(), // UDP is not connection-oriented
                NetEvent::Connected(_, _) => {}
                NetEvent::Disconnected(_) => (),
            }
        }
        buffered
    }

    pub fn broadcast(&mut self, buffer: Vec<u8>) {
        let message = Message {
            from: self.id.clone(),
            buffer,
        };
        let bin = bincode::serialize(&message).unwrap();
        match self.handler.network().send(self.endpoint, &bin) {
            message_io::network::SendStatus::Sent => {}
            message_io::network::SendStatus::ResourceNotAvailable => {}
            message_io::network::SendStatus::ResourceNotFound => {}
            message_io::network::SendStatus::MaxPacketSizeExceeded => {}
        }
    }
}
