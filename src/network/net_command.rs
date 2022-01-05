use libp2p::{
    floodsub::Topic,
    kad::{record::Key, GetRecordResult, PutRecordResult},
};
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum NetCommand {
    PutRecord {
        key: Key,
        value: Vec<u8>,
        sender: oneshot::Sender<PutRecordResult>,
    },

    GetRecord {
        key: Key,
        sender: oneshot::Sender<GetRecordResult>,
    },

    RemoveRecord {
        key: Key,
        sender: oneshot::Sender<()>,
    },

    Sub {
        topic: Topic,
        sender: oneshot::Sender<bool>,
    },

    Unsub {
        topic: Topic,
        sender: oneshot::Sender<bool>,
    },

    Publish {
        topic: Topic,
        payload: Vec<u8>,
        sender: oneshot::Sender<()>,
    },
}
