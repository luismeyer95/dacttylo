use super::input::InputResultRecord;
use std::error::Error;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

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
        record: &InputResultRecord,
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

    pub fn load_from_contents(
        &self,
        text: &str,
    ) -> Result<InputResultRecord, Box<dyn Error + Send + Sync>> {
        let filepath = self.derive_filepath(text);

        let mut file = std::fs::OpenOptions::new().read(true).open(filepath)?;

        let mut bytes = vec![];
        file.read_to_end(&mut bytes)?;
        let inputs: InputResultRecord = bincode::deserialize(&bytes)?;

        Ok(inputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::InputResult;
    use std::time::Duration;

    #[test]
    fn save_and_reload() {
        let manager = RecordManager::mount_dir("records").unwrap();
        let inputs: InputResultRecord =
            vec![(Duration::from_secs(0).into(), InputResult::Wrong('w'))]
                .into();

        manager.save("hello", &inputs).unwrap();
        let loaded_inputs = manager.load_from_contents("hello").unwrap();

        assert_eq!(loaded_inputs, inputs);
    }
}
