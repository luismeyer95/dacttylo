use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::{error::Error, time::Duration};
use thiserror::Error;

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

//////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputRecord {
    pub inputs: Vec<(Elapsed, char)>,
}

impl From<Vec<(Elapsed, char)>> for InputRecord {
    fn from(v: Vec<(Elapsed, char)>) -> Self {
        InputRecord { inputs: v }
    }
}

impl From<InputRecord> for Vec<(Elapsed, char)> {
    fn from(val: InputRecord) -> Self {
        val.inputs
    }
}

////////////////////////////////

pub struct InputRecorder {
    start: Instant,
    record: InputRecord,
}

impl InputRecorder {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            record: InputRecord {
                inputs: Default::default(),
            },
        }
    }

    pub fn push(&mut self, ch: char) {
        let elapsed = Instant::now().duration_since(self.start);
        self.record.inputs.push((elapsed.into(), ch));
    }

    pub fn record(&self) -> &InputRecord {
        &self.record
    }
}

////////////////////////////////

#[derive(Error, Debug)]
pub enum RecordManagerError {
    #[error("`{0}` is not a directory")]
    NotADirectory(String),
}

pub struct RecordManager<'dir> {
    directory: &'dir Path,
}

impl<'dir> RecordManager<'dir> {
    pub fn mount_dir(pathstr: &'dir str) -> Result<Self, RecordManagerError> {
        let path = Path::new(pathstr);
        if path.is_dir() {
            Ok(RecordManager { directory: path })
        } else {
            Err(RecordManagerError::NotADirectory(pathstr.to_string()))
        }
    }

    fn derive_filepath(&self, strbuf: &str) -> PathBuf {
        let hex = blake3::hash(strbuf.as_bytes()).to_hex();
        self.directory.join(&hex.as_str()[0..10])
    }

    pub fn save(
        &self,
        text: &str,
        record: &InputRecord,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let filepath = self.derive_filepath(text);

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(filepath)?;

        let serial = bincode::serialize(&record)?;
        file.write_all(&serial)?;

        Ok(())
    }

    pub fn load(
        &self,
        text: &str,
    ) -> Result<InputRecord, Box<dyn Error + Send + Sync>> {
        let filepath = self.derive_filepath(text);

        let mut file = std::fs::OpenOptions::new().read(true).open(filepath)?;

        let mut bytes = vec![];
        file.read_to_end(&mut bytes)?;
        let inputs: InputRecord = bincode::deserialize(&bytes)?;

        Ok(inputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_reload() {
        let manager = RecordManager::mount_dir("records").unwrap();
        let inputs: InputRecord =
            vec![(Duration::from_secs(0).into(), 'w')].into();

        manager.save("hello", &inputs).unwrap();
        let loaded_inputs = manager.load("hello").unwrap();

        assert_eq!(loaded_inputs, inputs);
    }
}
