use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};

use std::time::Duration;

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Elapsed {
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    duration: Duration,
}

impl From<Duration> for Elapsed {
    fn from(d: Duration) -> Self {
        Elapsed { duration: d }
    }
}

impl From<Elapsed> for Duration {
    fn from(e: Elapsed) -> Self {
        e.duration
    }
}
