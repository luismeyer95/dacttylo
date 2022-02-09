// use crate::{
//     events::AppEvent,
//     input::record::{Elapsed, InputRecord},
// };
// use std::error::Error;
// use tokio::sync::mpsc::{self, Sender};

// #[allow(unused_imports)]
// use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};

// pub fn new(
//     inputs: InputRecord,
// ) -> (GhostClient, impl Stream<Item = GhostInput>) {
//     let (tx, rx) = mpsc::channel::<GhostInput>(256);
//     (
//         GhostClient {
//             tx,
//             inputs: Some(inputs),
//         },
//         ReceiverStream::new(rx),
//     )
// }

// #[derive(Debug, Clone)]
// pub struct GhostInput(char);

// #[derive(Debug, Clone)]
// pub struct GhostClient {
//     tx: Sender<GhostInput>,
//     inputs: Option<InputRecord>,
// }

// impl GhostClient {
//     pub async fn start(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
//         if let Some(record) = self.inputs.take() {
//             let tx = self.tx.clone();
//             let inputs: Vec<(Elapsed, char)> = record.into();

//             tokio::spawn(async move {
//                 Self::replay_inputs(inputs, tx).await;
//             });
//         }
//         Ok(())
//     }

//     async fn replay_inputs(
//         inputs: Vec<(Elapsed, char)>,
//         tx: Sender<GhostInput>,
//     ) {
//         let start = std::time::Instant::now();

//         for (elapsed, char) in inputs {
//             let now = std::time::Instant::now();
//             let elapsed: std::time::Duration = elapsed.into();
//             let delta = elapsed.saturating_sub(now.duration_since(start));

//             tokio::time::sleep(delta).await;
//             tx.send(GhostInput(char)).await.unwrap();
//         }
//     }
// }

// impl From<GhostInput> for AppEvent {
//     fn from(c: GhostInput) -> Self {
//         AppEvent::GhostInput(c.0)
//     }
// }
