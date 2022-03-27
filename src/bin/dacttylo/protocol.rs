use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum ProtocolCommand {
    Input(char),
    Forfeit,
}
