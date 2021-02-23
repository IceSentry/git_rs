pub mod database;
pub mod workspace;

use std::path::PathBuf;

use anyhow::Result;

type ObjectId = String;

pub const GIT_FOLDER: &str = "git"; // TODO reset to .git

pub struct Refs {
    path: PathBuf,
}

impl Refs {
    pub fn head_path(&self) -> PathBuf {
        self.path.join("HEAD")
    }

    pub fn update_head(&self, object_id: ObjectId) -> Result<()> {
        Ok(std::fs::write(self.head_path(), &object_id)?)
    }

    pub fn read_head(&self) -> Result<ObjectId> {
        Ok(std::fs::read_to_string(self.head_path())?)
    }
}
