use anyhow::{Context, Result};

/// Converts a string of hexadecimal numbers to the equivalent Vec<u8>
pub fn serialize_oid(s: &str) -> Result<Vec<u8>> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).with_context(|| "decode_hex"))
        .collect()
}
