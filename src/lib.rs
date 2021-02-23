use std::{
    fs::{self, File},
    io::Write,
    iter::repeat_with,
    path::{Path, PathBuf},
};

use anyhow::Result;
use crypto::{digest::Digest, sha1::Sha1};
use flate2::{write::ZlibEncoder, Compression};

const MODE: &str = "100644";

pub enum Object {
    Blob(Vec<u8>),
    Tree(Vec<Entry>),
}

impl Object {
    pub fn serialize_type(&self) -> &str {
        match self {
            Object::Blob(_) => "blob",
            Object::Tree(_) => "tree",
        }
    }

    pub fn serialize_data(&self) -> Vec<u8> {
        match self {
            Object::Blob(data) => data.clone(),
            Object::Tree(entries) => {
                let mut entries = entries.to_vec();
                entries.sort_by(|a, b| a.name.cmp(&b.name));
                let mut tree = vec![];
                for entry in entries.iter() {
                    let mut entry_vec: Vec<u8> = format!("{} {}\0", MODE, entry.name.display())
                        .as_bytes()
                        .to_vec();
                    entry_vec.extend_from_slice(&entry.object_id.as_bytes()[..2]);
                    tree.extend_from_slice(&entry_vec);
                }
                tree
            }
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let data = self.serialize_data();
        let mut content = format!("{} {}\0", self.serialize_type(), data.len())
            .as_bytes()
            .to_vec();
        content.extend_from_slice(&data);
        content
    }
}

pub struct Database {
    pub path: PathBuf,
}

impl Database {
    pub fn store(&self, obj: Object) -> Result<String> {
        let object_id = hash(&obj);
        write(&self.path, &object_id, obj.serialize()).expect("Failed to write object to database");
        Ok(object_id)
    }
}

fn hash(obj: &Object) -> String {
    let mut hasher = Sha1::new();
    hasher.input(&obj.serialize());
    hasher.result_str()
}

fn write(path: &Path, object_id: &str, content: Vec<u8>) -> Result<()> {
    let object_path = path.join(&object_id[..2]).join(&object_id[2..]);
    let parent = object_path.parent().expect("Failed to get parent");

    let temp_path = parent.join(generate_temp_name());

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&content)?;
    let compressed = &encoder.finish()?;

    fs::create_dir_all(&parent)?;
    let mut file = File::create(&temp_path)?;
    file.write_all(compressed)?;

    fs::rename(temp_path, &object_path)?;

    Ok(())
}

/// Generates a random string of 6 alphanumerical characters
fn generate_temp_name() -> String {
    let s: String = repeat_with(fastrand::alphanumeric).take(6).collect();
    format!("tmp_obj_#{}", s)
}

#[derive(Clone)]
pub struct Entry {
    pub name: PathBuf,
    pub object_id: String,
}
