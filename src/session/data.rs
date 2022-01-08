use serde::{Deserialize, Serialize};

/// Session handle and context needed to join a session.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct SessionData {
    /// Used as the floodsub topic.
    pub session_id: String,

    /// Session specific data.
    pub metadata: Vec<u8>,
}
