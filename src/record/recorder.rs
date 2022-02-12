use super::input::InputResultRecord;
use crate::app::InputResult;
use std::time::Instant;

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
        let elapsed = Instant::now().duration_since(self.start);
        self.record.inputs.push((elapsed.into(), input));
    }

    pub fn record(&self) -> &InputResultRecord {
        &self.record
    }
}
