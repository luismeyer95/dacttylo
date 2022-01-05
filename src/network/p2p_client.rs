use super::NetCommand;
use libp2p::{
    floodsub::Topic,
    kad::{record::Key, GetRecordResult, PutRecordResult},
};
use std::error::Error;
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct P2PClient {
    sender: mpsc::Sender<NetCommand>,
}

impl P2PClient {
    pub fn new(sender: mpsc::Sender<NetCommand>) -> Self {
        Self { sender }
    }

    pub async fn put_record(
        &self,
        key: Key,
        value: impl Into<Vec<u8>>,
    ) -> Result<PutRecordResult, Box<dyn Error>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(NetCommand::PutRecord {
                key: key.clone(),
                value: value.into(),
                sender: tx,
            })
            .await?;

        Ok(rx.await?)
    }

    pub async fn get_record(&self, key: Key) -> Result<GetRecordResult, Box<dyn Error>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(NetCommand::GetRecord { key, sender: tx })
            .await?;

        Ok(rx.await?)
    }

    pub async fn remove_record(&self, key: Key) -> Result<(), Box<dyn Error>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(NetCommand::RemoveRecord { key, sender: tx })
            .await?;

        Ok(rx.await?)
    }

    pub async fn subscribe(&self, topic: Topic) -> Result<bool, Box<dyn Error>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(NetCommand::Sub { topic, sender: tx })
            .await?;

        Ok(rx.await?)
    }

    pub async fn unsubscribe(&self, topic: Topic) -> Result<bool, Box<dyn Error>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(NetCommand::Unsub { topic, sender: tx })
            .await?;

        Ok(rx.await?)
    }

    pub async fn publish(
        &self,
        topic: Topic,
        payload: impl Into<Vec<u8>>,
    ) -> Result<(), Box<dyn Error>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(NetCommand::Publish {
                topic,
                payload: payload.into(),
                sender: tx,
            })
            .await?;

        Ok(rx.await?)
    }
}
