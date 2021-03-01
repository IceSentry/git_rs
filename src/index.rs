use std::{
    cmp::min,
    collections::BTreeMap,
    convert::TryInto,
    fs::OpenOptions,
    io::{Read, Write},
    path::PathBuf,
    time::SystemTime,
};

use anyhow::{bail, Context, Result};

use crate::{
    database::{MODE_EXECUTABLE, MODE_REGULAR},
    lockfile::Lockfile,
    utils::deserialize_hash,
    HashWriter, Metadata, ObjectId,
};

const ENTRY_MAX_PATH_SIZE: usize = 0xfff;
const ENTRY_BLOCK_SIZE: usize = 8;
const VERSION: u32 = 2;
const HEADER_SIZE: usize = 12;
const CHECKSUM_SIZE: usize = 20;
const SIGNATURE: &str = "DIRC";
const ENTRY_MIN_SIZE: usize = 64;

pub struct Index {
    entries: BTreeMap<String, Entry>,
    lockfile: Lockfile,
    changed: bool,
}

impl Index {
    pub fn new(path: PathBuf) -> Self {
        Self {
            entries: BTreeMap::new(),
            lockfile: Lockfile::new(&path),
            changed: false,
        }
    }

    pub fn add(&mut self, path: String, object_id: ObjectId, metadata: &Metadata) -> Result<()> {
        let entry = Entry::new(path, object_id, metadata)?;
        self.insert_entry(entry);
        self.changed = true;
        Ok(())
    }

    fn insert_entry(&mut self, entry: Entry) {
        self.entries.insert(entry.path.clone(), entry);
    }

    pub fn write_updates(&mut self) -> Result<()> {
        if !self.changed {
            return self.lockfile.rollback();
        }

        let mut writer = ChecksumBuf::new(&self.lockfile);

        let mut header_bytes = vec![];
        header_bytes.extend_from_slice(b"DIRC");
        header_bytes.extend_from_slice(&VERSION.to_be_bytes());
        header_bytes.extend_from_slice(&(self.entries.len() as u32).to_be_bytes());
        writer.write(&header_bytes)?;

        for entry in self.entries.values() {
            writer.write(&entry.serialize()?)?;
        }

        writer.write_checksum()?;
        self.lockfile.commit()?;

        self.changed = false;

        Ok(())
    }

    pub fn load_for_update(&mut self) -> Result<()> {
        self.lockfile.hold_for_update()?;

        self.entries = Default::default();
        self.changed = false;

        let file = OpenOptions::new().read(true).open("foo.txt")?;
        let mut reader = ChecksumBuf::new(file);

        let data = reader.read(HEADER_SIZE)?;
        let signature = std::str::from_utf8(&data[..4])?;
        let version = u32::from_be_bytes(data[4..8].try_into()?);
        let count = u32::from_be_bytes(data[8..12].try_into()?);

        if signature != SIGNATURE {
            bail!(
                "Signature: expected '{}' but found '{}'",
                SIGNATURE,
                signature
            );
        }

        if version != VERSION {
            bail!("Version: expected '{}' but found '{}'", VERSION, version);
        }

        for _ in 0..count {
            let mut bytes = reader.read(ENTRY_MIN_SIZE)?;
            while bytes.last().context("load_for_update")? != &b'\0' {
                bytes.extend_from_slice(&reader.read(ENTRY_BLOCK_SIZE)?);
            }
            self.insert_entry(Entry::deserialize(bytes)?);
        }

        reader.verify_checksum()?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Entry {
    ctime: u32,
    ctime_nsec: u32,
    mtime: u32,
    mtime_nsec: u32,
    dev: u32,
    ino: u32,
    mode: u32,
    uid: u32,
    gid: u32,
    size: u32,
    oid: ObjectId,
    flags: u16,
    path: String,
}

impl Entry {
    // TODO add a #[cfg(unix)] version
    pub fn new(path: String, object_id: ObjectId, metadata: &Metadata) -> Result<Self> {
        let created = metadata.created.duration_since(SystemTime::UNIX_EPOCH)?;
        let modified = metadata.modified.duration_since(SystemTime::UNIX_EPOCH)?;

        Ok(Self {
            ctime: created.as_secs() as u32,
            ctime_nsec: created.as_nanos() as u32,
            mtime: modified.as_secs() as u32,
            mtime_nsec: modified.as_nanos() as u32,
            dev: metadata.device_id,
            // ino, uid, gid are set to 0 on windows because they don't have an equivalent
            ino: 0,
            mode: if metadata.is_executable {
                MODE_EXECUTABLE as u32
            } else {
                MODE_REGULAR as u32
            },
            uid: 0,
            gid: 0,
            size: metadata.len,
            oid: object_id,
            flags: min(path.len(), ENTRY_MAX_PATH_SIZE) as u16,
            path,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        let mut bytes = vec![];

        // 32 bits integers - metadata
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

        // 20 bytes string - oid
        let oid = crate::utils::serialize_hash(&self.oid)?;
        assert!(oid.len() == 20);
        bytes.extend_from_slice(&oid);

        // 16 bit unsigned number - flags
        bytes.extend_from_slice(&self.flags.to_be_bytes());

        // null terminated string - path
        bytes.extend_from_slice(self.path.as_bytes());
        bytes.push(0x0);

        // padding
        while bytes.len() % ENTRY_BLOCK_SIZE != 0 {
            bytes.push(0x0)
        }

        Ok(bytes)
    }

    fn deserialize(bytes: Vec<u8>) -> Result<Self> {
        fn read(bytes: &[u8], i: &mut usize, size: usize) -> Vec<u8> {
            let value = bytes[*i..*i + size].to_vec();
            *i += size;
            value
        }

        fn read_u32(bytes: &[u8], i: &mut usize) -> u32 {
            let bytes = read(bytes, i, std::mem::size_of::<u32>());
            u32::from_be_bytes(bytes.try_into().unwrap())
        }

        //WARN order is really important here
        let mut i = 0;
        let ctime = read_u32(&bytes, &mut i);
        let ctime_nsec = read_u32(&bytes, &mut i);
        let mtime = read_u32(&bytes, &mut i);
        let mtime_nsec = read_u32(&bytes, &mut i);
        let dev = read_u32(&bytes, &mut i);
        let ino = read_u32(&bytes, &mut i);
        let mode = read_u32(&bytes, &mut i);
        let uid = read_u32(&bytes, &mut i);
        let gid = read_u32(&bytes, &mut i);
        let size = read_u32(&bytes, &mut i);

        let oid = read(&bytes, &mut i, 20);
        let oid = deserialize_hash(&oid);

        let flags = u16::from_be_bytes(
            read(&bytes, &mut i, std::mem::size_of::<u16>())
                .try_into()
                .unwrap(),
        );

        let path_bytes = bytes[i..].split(|b| b == &0u8).next().unwrap();
        let path = std::str::from_utf8(path_bytes)?.to_string();

        Ok(Self {
            ctime,
            ctime_nsec,
            mtime,
            mtime_nsec,
            dev,
            ino,
            mode,
            uid,
            gid,
            size,
            oid,
            flags,
            path,
        })
    }
}

struct ChecksumBuf<F>
where
    F: Read + Write,
{
    buf: F,
    hash_writer: HashWriter,
}

impl<F> ChecksumBuf<F>
where
    F: Read + Write,
{
    fn new(buf: F) -> Self {
        Self {
            buf,
            hash_writer: HashWriter::new(),
        }
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.buf.write_all(data)?;
        self.hash_writer.write(data);
        Ok(())
    }

    fn write_checksum(&mut self) -> Result<()> {
        let checksum = self.hash_writer.finish();
        self.buf.write_all(checksum.as_bytes())?;
        Ok(())
    }

    fn read(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0; size];
        self.buf.read_exact(&mut buf)?;
        self.hash_writer.write(&buf);
        Ok(buf)
    }

    fn verify_checksum(&mut self) -> Result<()> {
        let mut buf = vec![0; CHECKSUM_SIZE];
        self.buf.read_exact(&mut buf)?;
        let sum = deserialize_hash(&buf);
        let hash = self.hash_writer.finish();
        if sum != hash {
            bail!("Checksum does not match value stored on disk")
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn entry_serialization() -> Result<()> {
        let mut hash_writer = HashWriter::new();
        hash_writer.write("entry_serialization".as_bytes());
        let hash = hash_writer.finish();

        let entry = Entry::new(
            "./temp".into(),
            hash,
            &Metadata {
                created: SystemTime::now(),
                modified: SystemTime::now(),
                accessed: SystemTime::now(),
                len: 0,
                is_executable: false,
                device_id: 0,
            },
        )?;

        let serialized_entry = Entry::deserialize(entry.serialize()?)?;

        assert!(serialized_entry.oid == entry.oid);
        assert!(serialized_entry.path == entry.path);

        Ok(())
    }
}
