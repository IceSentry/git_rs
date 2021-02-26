#![allow(clippy::expect_fun_call)]

pub mod database;
pub mod lockfile;
pub mod workspace;

use std::path::PathBuf;

use anyhow::{Context, Result};
use lockfile::Lockfile;

use std::fmt::{self, Display, Formatter};

use chrono::{DateTime, Utc};

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

        lockfile.write(&object_id)?;
        lockfile.write("\n")?;
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
