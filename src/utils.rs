use anyhow::Result;
use std::fmt::Write;

/// Converts a string of hexadecimal numbers to the equivalent vec of bytes
pub fn serialize_hash(s: &str) -> Result<Vec<u8>> {
    (0..s.len())
        .step_by(2)
        .map(|i| Ok(u8::from_str_radix(&s[i..i + 2], 16)?))
        .collect()
}

pub fn deserialize_hash(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for &b in data {
        write!(&mut s, "{:02x}", b).expect("hex encoding failed");
    }
    s
}
