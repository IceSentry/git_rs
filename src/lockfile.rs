use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};

pub struct Lockfile {
    file_path: PathBuf,
    lock_path: PathBuf,
    lock: Option<File>,
}

impl Lockfile {
    pub fn new(path: &Path) -> Self {
        Self {
            file_path: path.to_path_buf(),
            lock_path: path.with_extension("lock"),
            lock: None,
        }
    }

    pub fn hold_for_update(&mut self) -> Result<()> {
        if self.lock.is_none() {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&self.lock_path)?;

            self.lock = Some(file);
        }
        Ok(())
    }

    pub fn write(&mut self, data: &str) -> Result<()> {
        self.raise_on_stale_lock()?;
        if let Some(mut file) = self.lock.as_ref() {
            file.write_all(data.as_bytes())?;
        }
        Ok(())
    }

    pub fn commit(&mut self) -> Result<()> {
        self.raise_on_stale_lock()?;
        self.lock = None;
        std::fs::rename(&self.lock_path, &self.file_path)?;
        Ok(())
    }

    fn raise_on_stale_lock(&self) -> Result<()> {
        if self.lock.is_none() {
            bail!("Not holding lock on file: {}", self.lock_path.display())
        } else {
            Ok(())
        }
    }
}
