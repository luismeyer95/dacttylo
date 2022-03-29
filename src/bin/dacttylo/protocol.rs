use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum DacttyloCommand {
    Input(char),
    Forfeit,
}

#[derive(Serialize, Deserialize)]
pub struct DacttyloMetadata {
    pub syntax_name: String,
    pub text: String,
}
