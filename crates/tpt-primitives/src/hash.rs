//! Hash function used for identity derivation (ADR 0001: SHA-256).

use sha2::{Digest, Sha256};

/// Compute the SHA-256 digest of `data` (ADR 0001).
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Encode bytes as lowercase hex.
pub fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// Decode lowercase (or uppercase) hex into a fixed-size array.
pub fn from_hex<const N: usize>(hex: &str) -> Result<[u8; N], crate::error::CanonicalError> {
    let err = || crate::error::CanonicalError::BadHex(format!("expected {N} bytes as {} hex chars", N * 2));
    if hex.len() != N * 2 {
        return Err(err());
    }
    let mut out = [0u8; N];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = val(chunk[0]).ok_or_else(err)?;
        let lo = val(chunk[1]).ok_or_else(err)?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vectors() {
        // FIPS 180-4 test vectors.
        assert_eq!(
            to_hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            to_hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hex_round_trip() {
        let bytes = sha256(b"round-trip");
        let hex = to_hex(&bytes);
        assert_eq!(hex.len(), 64);
        assert_eq!(from_hex::<32>(&hex).unwrap(), bytes);
    }

    #[test]
    fn hex_rejects_bad_input() {
        assert!(from_hex::<4>(&"zz").is_err());
        assert!(from_hex::<4>(&"aabbcc").is_err());
    }
}
