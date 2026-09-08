//! Internal hashing and canonical-serialization helpers.

use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;

/// Computes the lowercase hexadecimal SHA-256 digest of `bytes`.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest.iter() {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Serializes `value` to compact JSON bytes for digesting.
///
/// Adapter-owned types serialize deterministically (fixed struct field order,
/// `BTreeMap` key order, no maps with non-string keys, no floats), so the
/// serialization cannot fail; the fallback keeps digesting total.
pub(crate) fn canonical_json_bytes<T: Serialize + ?Sized>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_default()
}

/// Deterministic canonical JSON digest of `value` as lowercase hex.
pub(crate) fn canonical_digest<T: Serialize + ?Sized>(value: &T) -> String {
    sha256_hex(&canonical_json_bytes(value))
}

#[cfg(test)]
mod tests {
    use super::canonical_digest;
    use super::sha256_hex;

    #[test]
    fn sha256_hex_matches_known_vectors() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn canonical_digest_is_deterministic() {
        assert_eq!(canonical_digest(&(1u8, "a")), canonical_digest(&(1u8, "a")));
        assert_ne!(canonical_digest(&(1u8, "a")), canonical_digest(&(1u8, "b")));
    }
}
