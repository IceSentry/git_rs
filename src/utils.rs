use std::path::{Component, Path, Prefix};

use anyhow::{Context, Result};

/// Converts a string of hexadecimal numbers to the equivalent Vec<u8>
pub fn serialize_oid(s: &str) -> Result<Vec<u8>> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).with_context(|| "decode_hex"))
        .collect()
}

/// On windows, this returns the drive letter, on linux it will only return 0
// TODO add a linux compatible version
pub fn get_drive(path: &Path) -> Result<u32> {
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
            Component::RootDir => 0,
            component => {
                dbg!(component);
                panic!("Component is not a prefix")
            }
        },
    )
}
