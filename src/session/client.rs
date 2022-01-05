use std::error::Error;

use libp2p::{
    floodsub::Topic,
    kad::{record::Key, PeerRecord, Record},
};

use crate::network::P2PClient;
use crate::session::{SessionCommand, SessionData};

type AsyncResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone)]
pub struct SessionClient {
    p2p_client: P2PClient,
    current_session_id: Option<String>,
}

impl SessionClient {
    pub fn new(p2p_client: P2PClient) -> Self {
        Self {
            p2p_client,
            current_session_id: None,
        }
    }

    fn get_session(&self) -> Result<&str, &'static str> {
        self.current_session_id
            .as_ref()
            .map(|s| s.as_str())
            .ok_or("Session not found")
    }

    pub async fn get_hosted_session_data(&mut self, host: &str) -> AsyncResult<SessionData> {
        let key = Key::new(&host);
        let err_str = format!("Could not find record `{:?}`", key);

        let mut result = self
            .p2p_client
            .get_record(key.clone())
            .await
            .expect("P2P client channel failure")
            .map_err(|_| err_str.clone())?;

        let PeerRecord {
            record: Record { value, .. },
            ..
        } = result.records.pop().ok_or(err_str)?;

        Ok(bincode::deserialize(&value)?)
    }

    pub async fn host_session(&mut self, host: &str, session_data: SessionData) -> AsyncResult<()> {
        let s_id = session_data.session_id.clone();
        self.join_session(s_id.into()).await?;

        let key = Key::new(&host);
        let value = bincode::serialize(&session_data)?;

        let result = self
            .p2p_client
            .put_record(key.clone(), value)
            .await
            .expect("P2P client channel failure");

        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(format!("Could not put record `{:?}`", key).into()),
        }
    }

    pub async fn stop_hosting_session(&mut self, host: &str) -> AsyncResult<()> {
        let err_p2p = "P2P client channel failure";

        self.leave_session().await?;

        self.p2p_client
            .remove_record(Key::new(&host))
            .await
            .expect(err_p2p);

        self.current_session_id = None;

        Ok(())
    }

    pub async fn join_session(&mut self, session_id: String) -> AsyncResult<bool> {
        let result = self
            .p2p_client
            .subscribe(Topic::new(session_id.clone()))
            .await
            .expect("P2P client channel failure");

        self.current_session_id = Some(session_id);

        Ok(result)
    }

    pub async fn leave_session(&mut self) -> AsyncResult<bool> {
        let current_session_id = self.get_session()?;

        let result = self
            .p2p_client
            .unsubscribe(Topic::new(current_session_id))
            .await
            .expect("P2P client channel failure");

        self.current_session_id = None;

        Ok(result)
    }

    pub async fn publish(&mut self, session_cmd: SessionCommand) -> AsyncResult<()> {
        let current_session_id = self.get_session()?;
        let payload = bincode::serialize(&session_cmd)?;

        Ok(self
            .p2p_client
            .publish(Topic::new(current_session_id), payload)
            .await
            .expect("P2P client channel failure"))
    }
}
