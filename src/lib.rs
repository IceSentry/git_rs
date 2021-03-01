#![allow(clippy::expect_fun_call)]

pub mod database;
pub mod index;
pub mod lockfile;
pub mod utils;
pub mod workspace;

use std::{
    fmt::{self, Display, Formatter},
    io::Write,
    path::PathBuf,
    time::SystemTime,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use crypto::{digest::Digest, sha1::Sha1};

use lockfile::Lockfile;

type ObjectId = String;

pub const GIT_FOLDER: &str = "git"; // TODO reset to .git

pub struct Author {
    pub name: String,
    pub email: String,
    pub time: DateTime<Utc>,
}

impl Display for Author {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{} <{}> {} {}",
            self.name,
            self.email,
            self.time.timestamp(),
            self.time.timezone()
        )
    }
}

pub struct Refs {
    path: PathBuf,
}

impl Refs {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn head_path(&self) -> PathBuf {
        self.path.join("HEAD")
    }

    pub fn update_head(&self, object_id: ObjectId) -> Result<()> {
        let mut lockfile = Lockfile::new(&self.head_path());

        lockfile.hold_for_update().with_context(|| {
            format!(
                "Could not acquire lock on file: {}",
                self.head_path().display()
            )
        })?;

        lockfile.write_all(&object_id.as_bytes())?;
        lockfile.write_all(b"\n")?;
        lockfile.commit()?;
        Ok(())
    }

    pub fn read_head(&self) -> Option<ObjectId> {
        if self.head_path().exists() {
            std::fs::read_to_string(self.head_path()).ok()
        } else {
            None
        }
    }
}

/// Computes the sha1 of the given data
pub fn hash(content: &[u8]) -> ObjectId {
    let mut hasher = HashWriter::new();
    hasher.write(content);
    hasher.finish()
}

/// Wrapper around Sha1
pub struct HashWriter {
    hasher: Sha1,
}

impl Default for HashWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl HashWriter {
    pub fn new() -> Self {
        Self {
            hasher: Sha1::new(),
        }
    }

    pub fn write(&mut self, bytes: &[u8]) {
        self.hasher.input(bytes);
    }

    pub fn finish(&mut self) -> String {
        self.hasher.result_str()
    }
}

pub struct Metadata {
    pub created: SystemTime,
    pub modified: SystemTime,
    pub accessed: SystemTime,
    pub len: u32,
    pub is_executable: bool,
    pub device_id: u32,
}
