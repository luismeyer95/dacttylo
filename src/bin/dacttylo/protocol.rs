use dacttylo::{
    session::{SessionClient, SessionCommand},
    utils::types::AsyncResult,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum ProtocolCommand {
    Input(char),
    Forfeit,
}

pub struct ProtocolClient {
    session_client: SessionClient,
}

impl ProtocolClient {
    pub fn new(session_client: SessionClient) -> ProtocolClient {
        ProtocolClient { session_client }
    }

    pub async fn publish(&mut self, cmd: ProtocolCommand) -> AsyncResult<()> {
        let payload = bincode::serialize(&cmd)?;
        self.session_client
            .publish(SessionCommand::Push(payload))
            .await;

        Ok(())
    }

    // pub fn parse(bytes: Vec<u8>) -> Option<ProtocolCommand> {
    //     // let session_command =
    // }
}
