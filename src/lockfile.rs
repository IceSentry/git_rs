use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
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

    pub fn commit(&mut self) -> Result<()> {
        self.raise_on_stale_lock()?;
        self.lock = None;
        std::fs::rename(&self.lock_path, &self.file_path)?;
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<()> {
        self.raise_on_stale_lock()?;
        std::fs::remove_file(self.lock_path.clone())?;
        self.lock = None;

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

impl Write for Lockfile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.raise_on_stale_lock() {
            Ok(_) => (),
            Err(err) => return Err(std::io::Error::new(std::io::ErrorKind::Other, err)),
        }

        if let Some(mut file) = self.lock.as_ref() {
            file.write(buf)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "There was no lockfile",
            ))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(mut file) = self.lock.as_ref() {
            file.flush()
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "There was no lockfile",
            ))
        }
    }
}

impl<'a> Write for &'a Lockfile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.raise_on_stale_lock() {
            Ok(_) => (),
            Err(err) => return Err(std::io::Error::new(std::io::ErrorKind::Other, err)),
        }

        if let Some(mut file) = self.lock.as_ref() {
            file.write(buf)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "There was no lockfile",
            ))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(mut file) = self.lock.as_ref() {
            file.flush()
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "There was no lockfile",
            ))
        }
    }
}

impl Read for Lockfile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(mut file) = self.lock.as_ref() {
            file.read(buf)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "There was no lockfile",
            ))
        }
    }
}

impl<'a> Read for &'a Lockfile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(mut file) = self.lock.as_ref() {
            file.read(buf)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "There was no lockfile",
            ))
        }
    }
}
