use crate::{
    events::AppEvent,
    input::record::{Elapsed, InputRecord},
};
use std::error::Error;
use tokio::sync::mpsc::{self, Sender};

#[allow(unused_imports)]
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};

pub fn new(inputs: InputRecord) -> (GhostClient, impl Stream<Item = char>) {
    let (tx, rx) = mpsc::channel::<char>(256);
    (
        GhostClient {
            tx,
            inputs: Some(inputs),
        },
        ReceiverStream::new(rx),
    )
}

#[derive(Debug, Clone)]
pub struct GhostClient {
    tx: Sender<char>,
    inputs: Option<InputRecord>,
}

impl GhostClient {
    pub async fn start(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if let Some(record) = self.inputs.take() {
            let tx = self.tx.clone();
            let inputs: Vec<(Elapsed, char)> = record.into();
            let start = std::time::Instant::now();

            tokio::spawn(async move {
                for (elapsed, char) in inputs {
                    let now = std::time::Instant::now();
                    let elapsed: std::time::Duration = elapsed.into();
                    let delta =
                        elapsed.saturating_sub(now.duration_since(start));
                    tokio::time::sleep(delta).await;
                    tx.send(char).await.unwrap();
                }
            });
        }
        Ok(())
    }
}

impl From<char> for AppEvent {
    fn from(c: char) -> Self {
        AppEvent::Ghost(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn send_a_single_tick() {}
}
