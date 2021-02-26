use std::{
    cmp::min,
    collections::HashMap,
    path::{Component, Path, PathBuf, Prefix},
    string::ToString,
    time::SystemTime,
};

use anyhow::Result;
use is_executable::IsExecutable;

use crate::{database::Mode, lockfile::Lockfile, ObjectId};

const ENTRY_MAX_PATH_SIZE: usize = 0xfff;
const ENTRY_BLOCK_SIZE: usize = 8;

pub struct Index {
    entries: HashMap<String, Entry>,
    lockfile: Lockfile,
}

impl Index {
    pub fn new(path: PathBuf) -> Self {
        Self {
            entries: HashMap::new(),
            lockfile: Lockfile::new(&path),
        }
    }

    pub fn add(
        &mut self,
        path: &Path,
        object_id: ObjectId,
        metadata: &std::fs::Metadata,
    ) -> Result<()> {
        let entry = Entry::new(path, object_id, metadata)?;
        self.entries.insert(path.display().to_string(), entry);
        Ok(())
    }

    pub fn write_updates(&mut self) -> Result<()> {
        self.lockfile.hold_for_update()?;

        let mut header_bytes: Vec<u8> = vec![];
        header_bytes.extend_from_slice(b"DIRC");
        header_bytes.extend_from_slice(&2u32.to_be_bytes()); // version no.
        header_bytes.extend_from_slice(&(self.entries.len() as u32).to_be_bytes());

        let mut hash_writer = crate::HashWriter::new();
        hash_writer.write(&header_bytes);
        for entry in self.entries.values() {
            hash_writer.write(&entry.serialize());
        }
        self.lockfile.write(&hash_writer.finish())?;
        self.lockfile.commit()?;

        Ok(())
    }
}

struct Entry {
    ctime: u32,
    ctime_nsec: u32,
    mtime: u32,
    mtime_nsec: u32,
    dev: u32,
    ino: u32,
    uid: u32,
    gid: u32,
    mode: u32,
    size: u32,
    oid: ObjectId,
    flags: u16,
    path: PathBuf,
}

impl Entry {
    pub fn new(path: &Path, object_id: ObjectId, metadata: &std::fs::Metadata) -> Result<Self> {
        let ctime = metadata.created()?.duration_since(SystemTime::UNIX_EPOCH)?;
        let mtime = metadata
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?;
        // ino, uid, gid are set to 0 on windows because they don't have an equivalent
        Ok(Self {
            ctime: ctime.as_secs() as u32,
            ctime_nsec: ctime.as_nanos() as u32,
            mtime: mtime.as_secs() as u32,
            mtime_nsec: mtime.as_nanos() as u32,
            dev: get_drive(path)?,
            ino: 0,
            uid: 0,
            gid: 0,
            mode: if path.is_executable() {
                Mode::Executable.to_string().parse::<u32>()?
            } else {
                Mode::Regular.to_string().parse::<u32>()?
            },
            size: metadata.len() as u32,
            oid: object_id,
            flags: min(path.display().to_string().len(), ENTRY_MAX_PATH_SIZE) as u16,
            path: path.to_path_buf(),
        })
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes = vec![];

        // 32 bits integers
        bytes.extend_from_slice(&self.ctime.to_be_bytes());
        bytes.extend_from_slice(&self.ctime_nsec.to_be_bytes());
        bytes.extend_from_slice(&self.mtime.to_be_bytes());
        bytes.extend_from_slice(&self.mtime_nsec.to_be_bytes());
        bytes.extend_from_slice(&self.dev.to_be_bytes());
        bytes.extend_from_slice(&self.ino.to_be_bytes());
        bytes.extend_from_slice(&self.mode.to_be_bytes());
        bytes.extend_from_slice(&self.uid.to_be_bytes());
        bytes.extend_from_slice(&self.gid.to_be_bytes());
        bytes.extend_from_slice(&self.size.to_be_bytes());

        // 20 bytes string
        let oid = self.oid.as_bytes();
        // FIXME this doesn't seem to be encoded properly
        assert!(oid.len() == 20);
        bytes.extend_from_slice(&self.oid.as_bytes());

        // 16 bit unsigned number
        bytes.extend_from_slice(&self.flags.to_be_bytes());

        // null terminated string
        bytes.extend_from_slice(&self.path.display().to_string().as_bytes());
        bytes.push(0);

        // padding
        while bytes.len() % ENTRY_BLOCK_SIZE != 0 {
            bytes.push(0)
        }

        bytes
    }
}

fn get_drive(path: &Path) -> Result<u32> {
    Ok(
        match path
            .canonicalize()?
            .components()
            .next()
            .expect("Failed to get first path component")
        {
            Component::Prefix(prefix_component) => match prefix_component.kind() {
                Prefix::VerbatimDisk(drive) => drive as u32 - 1,
                Prefix::Disk(drive) => drive as u32 - 1,
                _ => panic!("No drive detected in path"),
            },
            _ => panic!("Component is not a prefix"),
        },
    )
}
