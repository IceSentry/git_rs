use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::GIT_FOLDER;

const IGNORED: &[&str] = &[".git", GIT_FOLDER, "target"];

pub struct Workspace {
    path: PathBuf,
}

impl Workspace {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn list_files(&self) -> Result<Vec<PathBuf>> {
        list_files_recursive(&self.path)
    }

    pub fn read(&self, path: &Path) -> Vec<u8> {
        std::fs::read(&self.path.join(path)).expect(&format!("Failed to read {}", &path.display()))
    }

    pub fn file_metadata(&self, path: &Path) -> std::fs::Metadata {
        std::fs::metadata(self.path.join(path)).expect("Failed to read metadata")
    }
}

fn list_files_recursive(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![];
    for file in std::fs::read_dir(path)? {
        let file_path = file?.path();

        // ish
        if IGNORED.contains(&file_path.file_name().unwrap().to_str().unwrap()) {
            continue;
        }
        if file_path.is_dir() {
            files.extend_from_slice(&list_files_recursive(&file_path)?);
        } else {
            files.push(file_path);
        }
    }
    Ok(files)
}
