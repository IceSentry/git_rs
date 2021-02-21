use std::{
    fs::{self, File},
    io::{Read, Write},
};

use anyhow::{bail, Result};
use crypto::{digest::Digest, sha1::Sha1};
use flate2::{read::GzDecoder, write::ZlibEncoder, Compression};
use strum_macros::EnumString;

use crate::repository::Repository;

#[derive(EnumString)]
pub enum Type {
    Commit,
    Tree,
    Tag,
    Blob,
}

pub struct Object {
    object_type: Type,
    data: Vec<u8>,
}

impl Object {
    fn serialize_type(&self) -> &[u8] {
        match self.object_type {
            Type::Commit => b"commit",
            Type::Tree => b"tree",
            Type::Tag => b"tag",
            Type::Blob => b"blob",
        }
    }

    pub fn serialize(&self) -> &[u8] {
        match self.object_type {
            Type::Blob => &self.data,
            _ => unimplemented!(),
        }
    }

    pub fn deserialize(&mut self, data: Vec<u8>) {
        match self.object_type {
            Type::Blob => self.data = data,
            _ => unimplemented!(),
        }
    }
}

pub fn read(repo: &Repository, sha: &str) -> Result<Object> {
    let path = repo.dir.join("objects").join(&sha[..2]).join(&sha[2..]);

    let f = fs::read(path)?;

    let mut decoder = GzDecoder::new(&f[..]);
    let mut raw = String::new();
    decoder.read_to_string(&mut raw)?;

    // Read object type
    let object_type_index = raw
        .as_bytes()
        .iter()
        .position(|&x| x == b' ')
        .expect("Failed to find format");
    let object_type = &raw[..object_type_index];

    // Read and validate object size
    let size_index = raw
        .as_bytes()
        .iter()
        .skip(object_type_index)
        .position(|&x| x == b'\x00')
        .expect("Failed to find size");
    let size = &raw[object_type_index..size_index]
        .parse::<i32>()
        .expect("size is not a number");
    if *size as usize != raw.len() - size_index - 1 {
        bail!("Malformed object {}: bad length", sha)
    }

    Ok(Object {
        object_type: match object_type {
            "commit" => Type::Commit,
            "tree" => Type::Tree,
            "tag" => Type::Tag,
            "blob" => Type::Blob,
            _ => bail!("Unknown type {} for object {}", object_type, sha),
        },
        data: raw[size_index + 1..].as_bytes().to_vec(),
    })
}

pub fn write(repo: Repository, object: Object, actually_write: bool) -> Result<String> {
    let data = object.serialize();
    let mut result = object.serialize_type().to_vec(); // data.len() + b'\x00' + data;
    result.push(b' ');
    result.append(&mut data.len().to_ne_bytes().to_vec());
    result.push(b'\x00');
    result.append(&mut data.to_vec());

    let mut hasher = Sha1::new();
    hasher.input_str(std::str::from_utf8(&result)?);
    let sha = hasher.result_str();

    if actually_write {
        let path = repo.dir.join("objects").join(&sha[..2]).join(&sha[2..]);
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&result)?;
        File::create(path)?.write_all(&encoder.finish()?)?;
    }

    Ok(sha)
}

pub fn find<'a>(_repo: &'a Repository, name: &'a str, _object_type: Type) -> &'a str {
    name
}
