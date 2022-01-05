pub mod client;
pub use client::SessionClient;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Session handle and context needed to join a session.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct SessionData {
    /// Used as the floodsub topic.
    pub session_id: String,

    /// Session specific data.
    pub metadata: Vec<u8>,
}

/// Communication protocol for joining, starting,
/// and stopping sessions.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub enum SessionCommand {
    /// Announce participation to the session.
    /// The host will keep track of registered users
    /// by storing a PeerId => Username map which will
    /// be published on the topic prior to locking the
    /// session.
    Register { user: String },

    /// Command issued by the session host to communicate
    /// that registrations are closed and the session is
    /// scheduled to start at `session_start`
    LockSession {
        /// Map from peer_id to username
        /// (peer_id is not serializable, default to using a string)
        registered_users: HashMap<String, String>,

        /// Datetime of the scheduled session
        session_start: String,
    },

    /// Application specific push payload, what is sent
    /// is only relevant to the API user
    Push(Vec<u8>),

    /// Command issued by the session host to communicate
    /// the end of the session
    EndSession,
}
