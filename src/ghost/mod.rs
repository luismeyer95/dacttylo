use crate::{
    events::AppEvent,
    input::record::{Elapsed, InputRecord},
};
use std::error::Error;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct Ghost {
    inputs: Option<InputRecord>,
    tx: Sender<AppEvent>,
}

impl Ghost {
    pub fn new(inputs: InputRecord, tx: Sender<AppEvent>) -> Self {
        Self {
            inputs: Some(inputs),
            tx,
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if let Some(record) = self.inputs.take() {
            let tx = self.tx.clone();
            let inputs: Vec<(Elapsed, char)> = record.into();

            tokio::spawn(async move {
                Self::replay_inputs(inputs, tx).await;
            });
        }
        Ok(())
    }

    async fn replay_inputs(inputs: Vec<(Elapsed, char)>, tx: Sender<AppEvent>) {
        let start = std::time::Instant::now();

        for (elapsed, char) in inputs {
            let now = std::time::Instant::now();
            let elapsed: std::time::Duration = elapsed.into();
            let delta = elapsed.saturating_sub(now.duration_since(start));

            tokio::time::sleep(delta).await;
            tx.send(AppEvent::GhostInput(char)).await.unwrap();
        }
    }
}
