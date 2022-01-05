use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, FloodsubMessage},
    kad::{
        store::MemoryStore, GetRecordResult, Kademlia, KademliaEvent, PutRecordResult, QueryId,
        QueryResult, Quorum, Record,
    },
    mdns::{Mdns, MdnsEvent},
    swarm::SwarmEvent,
    NetworkBehaviour, Swarm,
};
use std::{collections::HashMap, error::Error};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

use super::{NetCommand, NetEvent};

// TODO: figure out how to get rid of this false positive

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "ComposedEvent")]
pub struct Behaviour {
    pub floodsub: Floodsub,
    pub kademlia: Kademlia<MemoryStore>,
    pub mdns: Mdns,
}

#[derive(Debug)]
pub enum ComposedEvent {
    Floodsub(FloodsubEvent),
    Kademlia(KademliaEvent),
    Mdns(MdnsEvent),
}

impl From<KademliaEvent> for ComposedEvent {
    fn from(event: KademliaEvent) -> Self {
        ComposedEvent::Kademlia(event)
    }
}

impl From<MdnsEvent> for ComposedEvent {
    fn from(event: MdnsEvent) -> Self {
        ComposedEvent::Mdns(event)
    }
}

impl From<FloodsubEvent> for ComposedEvent {
    fn from(event: FloodsubEvent) -> Self {
        ComposedEvent::Floodsub(event)
    }
}

pub struct EventLoop {
    swarm: Swarm<Behaviour>,
    command_receiver: ReceiverStream<NetCommand>,
    event_sender: mpsc::Sender<NetEvent>,

    pending_get_record: HashMap<QueryId, oneshot::Sender<GetRecordResult>>,
    pending_put_record: HashMap<QueryId, oneshot::Sender<PutRecordResult>>,
}

impl EventLoop {
    pub fn new(
        swarm: Swarm<Behaviour>,
        command_receiver: ReceiverStream<NetCommand>,
        event_sender: mpsc::Sender<NetEvent>,
    ) -> Self {
        Self {
            swarm,
            command_receiver,
            event_sender,
            pending_get_record: Default::default(),
            pending_put_record: Default::default(),
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.swarm.next() => {
                    let event = event.expect("Swarm stream ended unexpectedly");
                    self.handle_event(event).await;
                },
                command = self.command_receiver.next() => match command {
                    Some(c) => self.handle_command(c).await,
                    // Command channel closed, thus shutting down the network event loop
                    None=>  return,
                },
            }
        }
    }

    async fn handle_command(&mut self, command: NetCommand) {
        match command {
            NetCommand::GetRecord { key, sender } => {
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .get_record(&key, Quorum::One);

                self.pending_get_record.insert(query_id, sender);
            }

            NetCommand::PutRecord { key, value, sender } => {
                let record = Record {
                    key,
                    value,
                    publisher: None,
                    expires: None,
                };
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .put_record(record, Quorum::One)
                    .expect("Failed to store record locally");

                self.pending_put_record.insert(query_id, sender);
            }

            NetCommand::RemoveRecord { key, sender } => {
                self.swarm.behaviour_mut().kademlia.remove_record(&key);
                sender
                    .send(())
                    .expect("Unexpected closed P2P client receiver");
            }

            NetCommand::Sub { topic, sender } => {
                let result = self.swarm.behaviour_mut().floodsub.subscribe(topic);
                sender
                    .send(result)
                    .expect("Unexpected closed P2P client receiver");
            }

            NetCommand::Unsub { topic, sender } => {
                let result = self.swarm.behaviour_mut().floodsub.unsubscribe(topic);
                sender
                    .send(result)
                    .expect("Unexpected closed P2P client receiver");
            }

            NetCommand::Publish {
                topic,
                payload,
                sender,
            } => {
                self.swarm.behaviour_mut().floodsub.publish(topic, payload);
                sender
                    .send(())
                    .expect("Unexpected closed P2P client receiver");
            } // _ => {}
        }
    }

    #[allow(clippy::single_match)]
    async fn handle_event(
        &mut self,
        event: SwarmEvent<ComposedEvent, impl Error + Send + Sync + 'static>,
    ) {
        match event {
            SwarmEvent::Behaviour(event) => match event {
                ComposedEvent::Floodsub(e) => self.handle_floodsub_event(e).await,
                ComposedEvent::Kademlia(e) => self.handle_kademlia_event(e).await,
                ComposedEvent::Mdns(e) => self.handle_mdns_event(e).await,
            },

            // SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            //     println!("Connection established with {:?}", peer_id);
            // }

            // SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
            //     println!("Connection closed with {:?} because {:?}", peer_id, cause);
            // }
            _ => {}
        }
    }

    #[allow(clippy::single_match)]
    async fn handle_floodsub_event(&mut self, event: FloodsubEvent) {
        match event {
            FloodsubEvent::Message(FloodsubMessage {
                source,
                topics,
                data,
                ..
            }) => self
                .event_sender
                .send(NetEvent::TopicMessage {
                    source,
                    topics,
                    data,
                })
                .await
                .expect("Unexpected closed P2P client receiver"),

            // FloodsubEvent::Subscribed { peer_id, topic } => {
            //     println!("{:?} subscribed to topic {:?}", peer_id, topic);
            // }
            _ => {}
        }
    }

    async fn handle_mdns_event(&mut self, event: MdnsEvent) {
        let behaviour = self.swarm.behaviour_mut();
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, multiaddr) in list {
                    behaviour.floodsub.add_node_to_partial_view(peer);
                    behaviour.kademlia.add_address(&peer, multiaddr);
                    // println!("Discovered {:?}", peer);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, _multiaddr) in list {
                    if !behaviour.mdns.has_node(&peer) {
                        behaviour.floodsub.remove_node_from_partial_view(&peer);
                        // self.kademlia.remove_address(&peer, &multiaddr);
                    }
                }
            }
        }
    }

    #[allow(clippy::single_match)]
    async fn handle_kademlia_event(&mut self, event: KademliaEvent) {
        match event {
            KademliaEvent::OutboundQueryCompleted { result, id, .. } => match result {
                QueryResult::PutRecord(result) => {
                    let sender = self
                        .pending_put_record
                        .remove(&id)
                        .expect("Failed to retrieve pending put record operation");
                    sender
                        .send(result)
                        .expect("Unexpected closed P2P client receiver");
                }

                QueryResult::GetRecord(result) => {
                    let sender = self
                        .pending_get_record
                        .remove(&id)
                        .expect("Failed to retrieve pending get record operation");
                    sender
                        .send(result)
                        .expect("Unexpected closed P2P client receiver");
                }

                _ => {}
            },
            _ => {}
        }
    }
}
