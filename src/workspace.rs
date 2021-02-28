use std::{
    path::{Component, Path, PathBuf, Prefix},
    time::SystemTime,
};

use anyhow::{Context, Result};
use is_executable::IsExecutable;
use pathdiff::diff_paths;

use crate::{Metadata, GIT_FOLDER};

const IGNORED: &[&str] = &[".git", GIT_FOLDER, "target"];

pub struct Workspace {
    path: PathBuf,
}

impl Workspace {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn list_files(&self, path: Option<&Path>) -> Result<Vec<PathBuf>> {
        let path = match path {
            Some(path) => path,
            None => &self.path,
        };
        let mut files = vec![];

        if path.is_dir() {
            for file in std::fs::read_dir(path)? {
                let file_path = file?.path();
                // FIXME there has to be a better way to ignore some paths
                if IGNORED.contains(&file_path.file_name().unwrap().to_str().unwrap()) {
                    continue;
                }
                files.extend_from_slice(&self.list_files(Some(&file_path))?);
            }
        } else {
            files.push(diff_paths(path, &self.path).with_context(|| "Failed to get relative path")?)
        }

        Ok(files)
    }

    pub fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        Ok(std::fs::read(&self.path.join(path))?)
    }

    pub fn file_metadata(&self, path: &Path) -> Result<Metadata> {
        let fs_metadata = std::fs::metadata(self.path.join(path)).expect("Failed to read metadata");
        Ok(Metadata {
            // On linux this isn't supported so set it to beginning of time
            created: fs_metadata.created().unwrap_or(SystemTime::UNIX_EPOCH),
            modified: fs_metadata.modified()?,
            accessed: fs_metadata.accessed()?,
            device_id: 0, // Could use get_drive() but paths are hard
            len: fs_metadata.len() as u32,
            is_executable: path.is_executable(),
        })
    }
}

/// On windows, this returns the drive letter, on linux it will only return 0
// TODO add a linux compatible version that returns device_id
fn _get_drive(path: &Path) -> Result<u32> {
    let path = path.canonicalize()?;
    let mut components = path.components();

    Ok(
        match components
            .next()
            .expect("Failed to get first path component")
        {
            Component::Prefix(prefix_component) => match prefix_component.kind() {
                Prefix::VerbatimDisk(drive) => drive as u32 - 1,
                Prefix::Disk(drive) => drive as u32 - 1,
                _ => panic!("No drive detected in path"),
            },
            Component::RootDir => 0,
            _ => {
                panic!("Component is not a prefix")
            }
        },
    )
}
