use std::{
    fs::{self, File},
    io::Write,
    iter::repeat_with,
    path::PathBuf,
};

use anyhow::Result;
use crypto::{digest::Digest, sha1::Sha1};
use flate2::{write::ZlibEncoder, Compression};
use fs::metadata;

pub enum Object {
    Blob(Vec<u8>),
}

impl Object {
    pub fn serialize_type(&self) -> &[u8] {
        match self {
            Object::Blob(_) => b"blob",
        }
    }

    pub fn serialize_data(&self) -> &[u8] {
        match self {
            Object::Blob(data) => &data,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let data = self.serialize_data();
        let mut content = vec![];
        content.append(&mut self.serialize_type().to_vec());
        content.push(b' ');
        content.append(&mut data.len().to_ne_bytes().to_vec());
        content.push(b'\x00');
        content.append(&mut data.to_vec());

        content
    }
}

pub struct Database {
    pub path: PathBuf,
}

impl Database {
    pub fn store(&self, obj: Object) -> Result<()> {
        let content = obj.serialize();
        let mut hasher = Sha1::new();
        hasher.input(&content);
        let hash = hasher.result_str();

        write(&self.path, hash, content).expect("Failed to write object");

        Ok(())
    }
}

fn write(path: &PathBuf, hash: String, content: Vec<u8>) -> Result<()> {
    let object_path = path.join(&hash[..=1]).join(&hash[2..]);
    let temp_path = object_path.parent().unwrap().join(generate_temp_name());

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&content)
        .expect("Failed to write to encoder");
    let compressed = &encoder.finish().expect("Failed to compress content");

    fs::create_dir_all(&object_path.parent().unwrap()).expect(&format!(
        "Failed to create object_path at: {}",
        object_path.display()
    ));
    let mut file = File::create(&temp_path).expect(&format!(
        "Failed to create temp file at: {}",
        temp_path.display()
    ));
    file.write_all(compressed)
        .expect("Failed to write to temp file");

    let mut perms = fs::metadata(&temp_path)?.permissions();
    perms.set_readonly(false);
    fs::set_permissions(&temp_path, perms)?;

    fs::rename(temp_path, object_path).expect("Failed to rename temp file");

    Ok(())
}

fn generate_temp_name() -> String {
    let s: String = repeat_with(fastrand::alphanumeric).take(6).collect();
    format!("tmp_obj_#{}", s)
}
