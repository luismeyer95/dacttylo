use super::input::InputResultRecord;
use crate::app::InputResult;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct InputResultRecorder {
    start: Instant,
    record: InputResultRecord,
}

impl InputResultRecorder {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            record: InputResultRecord {
                inputs: Default::default(),
            },
        }
    }

    pub fn push(&mut self, input: InputResult) {
        let elapsed = self.elapsed();
        self.record.inputs.push((elapsed.into(), input));
    }

    pub fn record(&self) -> &InputResultRecord {
        &self.record
    }

    pub fn elapsed(&self) -> Duration {
        Instant::now().duration_since(self.start)
    }
}
