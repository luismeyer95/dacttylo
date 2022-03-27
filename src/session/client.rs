use std::{error::Error, time::Duration};

use bincode::deserialize;
use libp2p::{floodsub::Topic, kad::record::Key};
use rand::{distributions::Alphanumeric, Rng};

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
            .as_deref()
            .ok_or("Session not found")
    }

    pub async fn get_hosted_sessions(
        &mut self,
        host: &str,
    ) -> AsyncResult<Vec<SessionData>> {
        let key = Key::new(&host);
        let err_str = format!("Could not find record `{:?}`", key);

        let result = self
            .p2p_client
            .get_record(key.clone())
            .await
            .expect("P2P client channel failure")
            .map_err(|_| err_str.clone())?;

        let session_list: Vec<SessionData> = result
            .records
            .iter()
            .filter_map(|peer_record| {
                deserialize(&peer_record.record.value).ok()
            })
            .collect();

        Ok(session_list)
    }

    pub async fn await_session_for_host(
        &mut self,
        host: impl AsRef<str>,
    ) -> SessionData {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Ok(session_list) =
                self.get_hosted_sessions(host.as_ref()).await
            {
                if let Some(session) = session_list.first() {
                    return session.clone();
                }
            }
        }
    }

    pub async fn host_session(
        &mut self,
        host: &str,
        metadata: Vec<u8>,
    ) -> AsyncResult<()> {
        let session_id: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        self.join_session(session_id.clone()).await?;

        let key = Key::new(&host);
        let value = bincode::serialize(&SessionData {
            session_id,
            metadata,
        })?;

        let result = self
            .p2p_client
            .put_record(key.clone(), value)
            .await
            .expect("P2P client channel failure");

        // match result {
        //     Ok(_) => Ok(()),
        //     Err(_) => Err(format!("Could not put record `{:?}`", key).into()),
        // }
        Ok(())
    }

    pub async fn stop_hosting_session(
        &mut self,
        host: &str,
    ) -> AsyncResult<()> {
        self.leave_session().await?;

        // This will only remove the record previously set by the local peer if any
        self.p2p_client
            .remove_record(Key::new(&host))
            .await
            .expect("P2P client channel failure");

        self.current_session_id = None;

        Ok(())
    }

    pub async fn join_session(
        &mut self,
        session_id: String,
    ) -> AsyncResult<bool> {
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

    pub async fn publish(
        &mut self,
        session_cmd: SessionCommand,
    ) -> AsyncResult<()> {
        let current_session_id = self.get_session()?;
        let payload = bincode::serialize(&session_cmd)?;

        self.p2p_client
            .publish(Topic::new(current_session_id), payload)
            .await
            .expect("P2P client channel failure");

        Ok(())
    }
}
