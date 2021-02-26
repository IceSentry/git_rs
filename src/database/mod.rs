pub mod blob;
pub mod commit;
pub mod tree;

use std::{
    fs::{self, File},
    io::Write,
    iter::repeat_with,
    path::PathBuf,
};

use anyhow::Result;
use crypto::{digest::Digest, sha1::Sha1};
use flate2::{write::ZlibEncoder, Compression};

use crate::ObjectId;

pub trait Object {
    fn serialize_type(&self) -> &str;
    fn serialize_data(&self) -> Vec<u8>;

    fn serialize(&self) -> Vec<u8> {
        let data = self.serialize_data();
        let mut content = format!("{} {}\0", self.serialize_type(), data.len())
            .as_bytes()
            .to_vec();
        content.extend_from_slice(&data);
        content
    }

    /// Computes the sha1 of the serialized content of the object
    fn object_id(&self) -> ObjectId {
        let content = self.serialize();
        hash(&content)
    }
}

pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn store<O>(&self, object: &O) -> Result<ObjectId>
    where
        O: Object,
    {
        // We don't use object.object_id() here because it would call serialize() twice in a row
        let content = object.serialize();
        let object_id = hash(&content);
        self.write(&object_id, content)?;
        Ok(object_id)
    }

    /// Writes the given object to the file system
    // TODO should this use the lockfile?
    pub fn write(&self, object_id: &str, content: Vec<u8>) -> Result<()> {
        let object_path = self.path.join(&object_id[..2]).join(&object_id[2..]);
        if object_path.exists() {
            return Ok(());
        }
        let dirname = object_path.parent().expect("Failed to get parent");
        let temp_path = dirname.join(generate_temp_name());

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&content)?;
        let compressed = &encoder.finish()?;

        fs::create_dir_all(&dirname)?;
        let mut file = File::create(&temp_path)?;
        file.write_all(compressed)?;

        println!("Writing {}", object_path.display());
        fs::rename(temp_path, &object_path)?;

        Ok(())
    }
}

/// Computes the sha1 of the given data
pub fn hash(content: &[u8]) -> ObjectId {
    let mut hasher = Sha1::new();
    hasher.input(content);
    hasher.result_str()
}

/// Generates a random string of 6 alphanumerical characters
fn generate_temp_name() -> String {
    let s: String = repeat_with(fastrand::alphanumeric).take(6).collect();
    format!("tmp_obj_#{}", s)
}