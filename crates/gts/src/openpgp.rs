// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Minimal OpenPGP reader for Ed25519 armored public/secret keys (§9.2).
//!
//! This mirrors the Python `gts.openpgp` reference: it parses only the
//! unencrypted armored public-key certificates and secret-key blocks GPG emits
//! for Ed25519 (OpenPGP algorithm 22) keys, extracting raw key material and
//! computing the v4 fingerprint so GTS tooling can sign or show the embedded
//! transport key without shelling out to `gpg`. Everything else (other
//! algorithms, encrypted secret keys, v5/v6 packets) is rejected with a clear
//! error.

use ed25519_dalek::SigningKey;
use sha1::{Digest, Sha1};

/// OpenPGP public-key algorithm id for EdDSA (RFC 9580 §9.1).
const ED25519_ALGO: u8 = 22;
/// The curve OID GPG writes for the Ed25519 signing curve (`1.3.6.1.4.1.11591.15.1`).
const ED25519_OID: [u8; 9] = [0x2b, 0x06, 0x01, 0x04, 0x01, 0xda, 0x47, 0x0f, 0x01];

/// An error parsing an armored OpenPGP key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenPgpError(pub String);

impl std::fmt::Display for OpenPgpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for OpenPgpError {}

type Result<T> = std::result::Result<T, OpenPgpError>;

fn err<T>(msg: &str) -> Result<T> {
    Err(OpenPgpError(msg.to_string()))
}

/// The parsed transport key: the raw Ed25519 public key plus its v4 fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportKey {
    /// The 32-byte raw Ed25519 public key (the `0x40` MPI marker stripped).
    pub raw_public: [u8; 32],
    /// Uppercase 40-hex-character OpenPGP v4 fingerprint.
    pub fingerprint: String,
}

/// Signing material loaded from an unencrypted OpenPGP Ed25519 secret-key block.
// `SigningKey`'s `Debug` impl redacts the secret scalar, so deriving is safe here.
#[derive(Clone, Debug)]
pub struct OpenPgpSigningKey {
    signing_key: SigningKey,
    kid: String,
    fingerprint: String,
    raw_public: [u8; 32],
}

impl OpenPgpSigningKey {
    /// Ed25519 signing key extracted from the OpenPGP secret-key packet.
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    /// COSE key id to use for signatures.
    pub fn kid(&self) -> &str {
        &self.kid
    }

    /// OpenPGP v4 fingerprint of the public-key material.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Raw 32-byte Ed25519 public key derived from the OpenPGP public material.
    pub fn raw_public(&self) -> &[u8; 32] {
        &self.raw_public
    }

    /// Consume the wrapper and return the key material used by [`crate::writer::Writer`].
    pub fn into_parts(self) -> (SigningKey, String) {
        (self.signing_key, self.kid)
    }
}

/// Decode the packet bytes from an ASCII-armored OpenPGP block.
fn strip_armor(text: &str) -> Result<Vec<u8>> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|l| l.starts_with("-----BEGIN PGP"));
    let Some(start) = start else {
        return err("missing armor BEGIN line");
    };
    let end = lines
        .iter()
        .enumerate()
        .position(|(i, l)| i > start && l.starts_with("-----END PGP"));
    let Some(end) = end else {
        return err("missing armor END line");
    };

    let mut idx = start + 1;
    // Skip optional armor headers (Comment, Version, …) up to the blank line.
    while idx < end && !lines[idx].trim().is_empty() {
        if lines[idx].contains(':') {
            idx += 1;
        } else {
            break;
        }
    }

    let mut body = String::new();
    while idx < end {
        let line = lines[idx];
        if line.starts_with('=') {
            break; // CRC-24 checksum line — end of the base64 body.
        }
        if !line.is_empty() {
            body.push_str(line);
        }
        idx += 1;
    }
    if body.is_empty() {
        return err("empty armor body");
    }
    b64_decode(&body)
}

/// Decode a base64 string (standard alphabet, no line breaks) without pulling
/// in an external crate — the armor body is small.
fn b64_decode(s: &str) -> Result<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for &c in s.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let Some(v) = val(c) else {
            return err("invalid base64 armor body");
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

/// Read an OpenPGP multi-precision integer; returns `(big_endian_bytes, next_offset)`.
fn read_mpi(data: &[u8], offset: usize) -> Result<(Vec<u8>, usize)> {
    if offset + 2 > data.len() {
        return err("truncated MPI length");
    }
    let bits = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
    let length = bits.div_ceil(8);
    let end = offset + 2 + length;
    if end > data.len() {
        return err("truncated MPI payload");
    }
    Ok((data[offset + 2..end].to_vec(), end))
}

/// Parse one OpenPGP packet; returns `(tag, body, next_offset)`.
/// Supports both old- and new-format headers.
fn next_packet(data: &[u8], mut offset: usize) -> Result<(u8, Vec<u8>, usize)> {
    if offset >= data.len() {
        return err("truncated packet header");
    }
    let header = data[offset];
    if header & 0x80 == 0 {
        return err("invalid packet tag octet");
    }

    let tag;
    let length;
    if header & 0x40 != 0 {
        // New-format packet.
        tag = header & 0x3f;
        offset += 1;
        if offset >= data.len() {
            return err("truncated new-format length octet");
        }
        let lo = data[offset];
        if lo < 192 {
            length = lo as usize;
            offset += 1;
        } else if lo < 224 {
            if offset + 1 >= data.len() {
                return err("truncated new-format 2-octet length");
            }
            length = (((lo as usize) - 192) << 8) + data[offset + 1] as usize + 192;
            offset += 2;
        } else if lo == 255 {
            if offset + 4 >= data.len() {
                return err("truncated new-format 4-octet length");
            }
            length = u32::from_be_bytes([
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
            ]) as usize;
            offset += 5;
        } else {
            return err("partial body lengths are not supported");
        }
    } else {
        // Old-format packet.
        tag = (header >> 2) & 0x0f;
        let length_type = header & 0x03;
        offset += 1;
        match length_type {
            0 => {
                if offset >= data.len() {
                    return err("truncated old-format length octet");
                }
                length = data[offset] as usize;
                offset += 1;
            }
            1 => {
                if offset + 1 >= data.len() {
                    return err("truncated old-format 2-octet length");
                }
                length = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                offset += 2;
            }
            2 => {
                if offset + 3 >= data.len() {
                    return err("truncated old-format 4-octet length");
                }
                length = u32::from_be_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]) as usize;
                offset += 4;
            }
            _ => return err("indeterminate-length packets are not supported"),
        }
    }

    let end = offset + length;
    if end > data.len() {
        return err("packet body exceeds input");
    }
    Ok((tag, data[offset..end].to_vec(), end))
}

/// Iterate every `(tag, body)` packet in the de-armored data.
fn iter_packets(data: &[u8]) -> Result<Vec<(u8, Vec<u8>)>> {
    let mut packets = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        let (tag, body, next) = next_packet(data, offset)?;
        packets.push((tag, body));
        offset = next;
    }
    Ok(packets)
}

/// Parse the OID and raw key from a v4 public-key packet body; returns
/// `(raw_public_key, end_offset_of_public_material)`.
fn parse_ed25519_public_material(body: &[u8]) -> Result<([u8; 32], usize)> {
    if body.len() < 6 || body[0] != 4 {
        return err("only OpenPGP v4 public keys are supported");
    }
    if body[5] != ED25519_ALGO {
        return Err(OpenPgpError(format!(
            "unsupported public-key algorithm {}",
            body[5]
        )));
    }
    let mut offset = 6;
    if offset >= body.len() {
        return err("truncated public-key packet");
    }
    let oid_len = body[offset] as usize;
    offset += 1;
    if offset + oid_len > body.len() {
        return err("truncated OID");
    }
    let oid = &body[offset..offset + oid_len];
    offset += oid_len;
    if oid != ED25519_OID {
        return Err(OpenPgpError(format!(
            "unsupported curve OID {}",
            crate::wire::hex(oid)
        )));
    }

    let (mpi, end) = read_mpi(body, offset)?;
    // GPG encodes the Ed25519 public key as a 33-byte MPI (`0x40 || 32-byte key`);
    // a bare 32-byte MPI is also valid when the high bit is clear.
    let raw: [u8; 32] = match mpi.len() {
        33 => mpi[1..].try_into().expect("33-1 == 32"),
        32 => mpi[..].try_into().expect("len checked"),
        n => {
            return Err(OpenPgpError(format!(
                "unexpected Ed25519 public MPI length {n}"
            )))
        }
    };
    Ok((raw, end))
}

fn secret_mpi_to_seed(mpi: &[u8]) -> Result<[u8; 32]> {
    let bytes = if mpi.first() == Some(&0) {
        &mpi[1..]
    } else {
        mpi
    };
    if bytes.len() > 32 {
        return Err(OpenPgpError(format!(
            "unexpected Ed25519 secret MPI length {}",
            mpi.len()
        )));
    }
    let mut raw = [0u8; 32];
    raw[32 - bytes.len()..].copy_from_slice(bytes);
    Ok(raw)
}

fn checksum16(data: &[u8]) -> u16 {
    data.iter()
        .fold(0u16, |sum, byte| sum.wrapping_add(u16::from(*byte)))
}

/// Parse the secret scalar from a v4 Ed25519 secret-key packet body.
fn parse_ed25519_secret_material(body: &[u8]) -> Result<([u8; 32], SigningKey, usize)> {
    let (raw_public, pub_end) = parse_ed25519_public_material(body)?;
    let mut offset = pub_end;

    if offset >= body.len() {
        return err("truncated secret-key packet");
    }
    let s2k_usage = body[offset];
    offset += 1;
    if s2k_usage != 0 {
        return err(
            "encrypted secret keys are not supported; export an unencrypted Ed25519 secret key",
        );
    }

    let secret_start = offset;
    let (mpi, next) = read_mpi(body, offset)?;
    if next + 2 != body.len() {
        return err("unsupported secret-key packet structure");
    }
    let expected = u16::from_be_bytes([body[next], body[next + 1]]);
    let actual = checksum16(&body[secret_start..next]);
    if actual != expected {
        return err("OpenPGP secret-key checksum mismatch");
    }

    let raw_secret = secret_mpi_to_seed(&mpi)?;
    let signing_key = SigningKey::from_bytes(&raw_secret);
    if signing_key.verifying_key().to_bytes() != raw_public {
        return err("OpenPGP secret key does not match public key material");
    }

    Ok((raw_public, signing_key, pub_end))
}

/// Compute the OpenPGP v4 fingerprint of a public-key packet body.
///
/// `SHA-1(0x99 || u16-be(len(body)) || body)`, uppercased. SHA-1 is mandated by
/// RFC 4880 for v4 fingerprints; it is not used here as a security primitive.
fn fingerprint(pub_key_body: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update([0x99]);
    hasher.update((pub_key_body.len() as u16).to_be_bytes());
    hasher.update(pub_key_body);
    let digest = hasher.finalize();
    crate::wire::hex(&digest).to_uppercase()
}

/// Parse an armored OpenPGP certificate into its raw Ed25519 key + v4 fingerprint.
///
/// Accepts either a public-key certificate (tag 6) or an unencrypted secret-key
/// block (tag 5); the fingerprint always covers only the public material.
pub fn parse_transport_key(armored: &str) -> Result<TransportKey> {
    let data = strip_armor(armored)?;
    for (tag, body) in iter_packets(&data)? {
        let (raw, pub_body): ([u8; 32], Vec<u8>) = match tag {
            6 => {
                let (raw, _) = parse_ed25519_public_material(&body)?;
                (raw, body)
            }
            5 => {
                let (raw, end) = parse_ed25519_public_material(&body)?;
                (raw, body[..end].to_vec())
            }
            _ => continue,
        };
        return Ok(TransportKey {
            raw_public: raw,
            fingerprint: fingerprint(&pub_body),
        });
    }
    err("no public-key packet found")
}

/// Parse an armored unencrypted OpenPGP Ed25519 secret key into signing material.
///
/// When `kid_override` is `None`, the COSE key id defaults to the OpenPGP v4
/// fingerprint of the secret key's public material, matching the Python
/// `Signer.from_gpg_secret_key` behavior.
pub fn parse_secret_signing_key(
    armored: &str,
    kid_override: Option<&str>,
) -> Result<OpenPgpSigningKey> {
    let data = strip_armor(armored)?;
    for (tag, body) in iter_packets(&data)? {
        if tag != 5 {
            continue;
        }
        let (raw_public, signing_key, pub_end) = parse_ed25519_secret_material(&body)?;
        let fingerprint = fingerprint(&body[..pub_end]);
        let kid = kid_override.unwrap_or(&fingerprint).to_string();
        return Ok(OpenPgpSigningKey {
            signing_key,
            kid,
            fingerprint,
            raw_public,
        });
    }
    err("no secret-key packet found")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn vectors_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vectors/openpgp")
    }

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(vectors_dir().join(name)).unwrap()
    }

    fn json_string_field(raw: &str, key: &str) -> String {
        let needle = format!("\"{key}\": \"");
        let start = raw
            .find(&needle)
            .unwrap_or_else(|| panic!("missing JSON string field {key:?}"))
            + needle.len();
        let mut out = String::new();
        let mut chars = raw[start..].chars();
        while let Some(ch) = chars.next() {
            match ch {
                '"' => return out,
                '\\' => match chars.next().expect("unterminated JSON escape") {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'b' => out.push('\u{0008}'),
                    'f' => out.push('\u{000c}'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        let mut hex = String::new();
                        for _ in 0..4 {
                            hex.push(chars.next().expect("truncated JSON unicode escape"));
                        }
                        let value =
                            u32::from_str_radix(&hex, 16).expect("invalid JSON unicode escape");
                        out.push(char::from_u32(value).expect("invalid JSON unicode scalar"));
                    }
                    escape => panic!("unsupported JSON escape {escape:?}"),
                },
                ch => out.push(ch),
            }
        }
        panic!("unterminated JSON string field {key:?}");
    }

    fn b64_encode(data: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in data.chunks(3) {
            let b0 = chunk[0];
            let b1 = chunk.get(1).copied().unwrap_or(0);
            let b2 = chunk.get(2).copied().unwrap_or(0);
            out.push(ALPHABET[(b0 >> 2) as usize] as char);
            out.push(ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
            if chunk.len() > 1 {
                out.push(ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
            } else {
                out.push('=');
            }
            if chunk.len() > 2 {
                out.push(ALPHABET[(b2 & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
        }
        out
    }

    fn encode_packet(tag: u8, body: &[u8]) -> Vec<u8> {
        let mut out = vec![0xc0 | tag];
        let len = body.len();
        if len < 192 {
            out.push(len as u8);
        } else if len < 8384 {
            let encoded = len - 192;
            out.push(((encoded >> 8) as u8) + 192);
            out.push(encoded as u8);
        } else {
            out.push(255);
            out.extend_from_slice(&(len as u32).to_be_bytes());
        }
        out.extend_from_slice(body);
        out
    }

    fn armor_private_key(data: &[u8]) -> String {
        let b64 = b64_encode(data);
        let mut wrapped = String::new();
        for line in b64.as_bytes().chunks(64) {
            wrapped.push_str(std::str::from_utf8(line).unwrap());
            wrapped.push('\n');
        }
        format!("-----BEGIN PGP PRIVATE KEY BLOCK-----\n\n{wrapped}-----END PGP PRIVATE KEY BLOCK-----\n")
    }

    fn secret_packet_body() -> Vec<u8> {
        let data = strip_armor(&fixture("test_key.sec.asc")).unwrap();
        iter_packets(&data)
            .unwrap()
            .into_iter()
            .find_map(|(tag, body)| (tag == 5).then_some(body))
            .unwrap()
    }

    fn mutated_secret_armor(mut mutate: impl FnMut(&mut Vec<u8>)) -> String {
        let mut body = secret_packet_body();
        mutate(&mut body);
        armor_private_key(&encode_packet(5, &body))
    }

    #[test]
    fn parses_frozen_vector() {
        let raw = std::fs::read_to_string(vectors_dir().join("test-key.json")).unwrap();
        let armored = json_string_field(&raw, "armored");
        let key = parse_transport_key(&armored).unwrap();
        assert_eq!(
            crate::wire::hex(&key.raw_public),
            json_string_field(&raw, "raw_pub")
        );
        assert_eq!(key.fingerprint, json_string_field(&raw, "fingerprint"));
        assert_eq!(
            crate::emojihash::emojihash(&key.raw_public, 11),
            json_string_field(&raw, "emojihash")
        );
    }

    #[test]
    fn rejects_non_pgp() {
        assert!(parse_transport_key("not a key").is_err());
    }

    #[test]
    fn parses_secret_signing_key_with_default_fingerprint_kid() {
        let public_armor = fixture("test_key.pub.asc");
        let secret_armor = fixture("test_key.sec.asc");
        let expected_fingerprint = fixture("test_key.fingerprint").trim().to_string();

        let transport = parse_transport_key(&public_armor).unwrap();
        let signer = parse_secret_signing_key(&secret_armor, None).unwrap();

        assert_eq!(signer.kid(), expected_fingerprint);
        assert_eq!(signer.fingerprint(), expected_fingerprint);
        assert_eq!(signer.raw_public(), &transport.raw_public);
        assert_eq!(
            signer.signing_key().verifying_key().to_bytes(),
            transport.raw_public
        );
    }

    #[test]
    fn parses_secret_signing_key_with_kid_override() {
        let signer =
            parse_secret_signing_key(&fixture("test_key.sec.asc"), Some("did:example:test"))
                .unwrap();

        assert_eq!(signer.kid(), "did:example:test");
        assert_eq!(signer.fingerprint(), fixture("test_key.fingerprint").trim());
    }

    #[test]
    fn secret_signing_key_rejects_public_armor() {
        let err = parse_secret_signing_key(&fixture("test_key.pub.asc"), None)
            .expect_err("public armor is rejected as signing material");
        assert!(err.0.contains("no secret-key packet"));
    }

    #[test]
    fn secret_signing_key_rejects_encrypted_secret_packets() {
        let armor = mutated_secret_armor(|body| {
            let (_, pub_end) = parse_ed25519_public_material(body).unwrap();
            body[pub_end] = 254;
        });

        let err =
            parse_secret_signing_key(&armor, None).expect_err("encrypted secret key is rejected");
        assert!(err.0.contains("encrypted secret keys are not supported"));
    }

    #[test]
    fn secret_mpi_to_seed_left_pads_short_mpis() {
        let seed = secret_mpi_to_seed(&[0x01, 0x23]).unwrap();
        assert_eq!(&seed[..30], &[0u8; 30]);
        assert_eq!(&seed[30..], &[0x01, 0x23]);
    }

    #[test]
    fn secret_signing_key_rejects_bad_checksum() {
        let armor = mutated_secret_armor(|body| {
            let last = body.last_mut().unwrap();
            *last ^= 0x01;
        });

        let err = parse_secret_signing_key(&armor, None)
            .expect_err("bad secret-key checksum is rejected");
        assert!(err.0.contains("checksum mismatch"));
    }

    #[test]
    fn secret_signing_key_rejects_unsupported_trailer_structure() {
        let armor = mutated_secret_armor(|body| {
            body.push(0);
        });

        let err = parse_secret_signing_key(&armor, None)
            .expect_err("unexpected secret-key trailer is rejected");
        assert!(err.0.contains("unsupported secret-key packet structure"));
    }

    #[test]
    fn secret_signing_key_rejects_public_secret_mismatch() {
        let armor = mutated_secret_armor(|body| {
            let (_, pub_end) = parse_ed25519_public_material(body).unwrap();
            body[pub_end - 1] ^= 0x01;
        });

        let err = parse_secret_signing_key(&armor, None)
            .expect_err("mismatched public and secret material is rejected");
        assert!(err.0.contains("does not match public key material"));
    }

    #[test]
    fn secret_signing_key_rejects_unsupported_algorithms_and_versions() {
        let wrong_algorithm = mutated_secret_armor(|body| {
            body[5] = 1;
        });
        let err = parse_secret_signing_key(&wrong_algorithm, None)
            .expect_err("unsupported algorithm is rejected");
        assert!(err.0.contains("unsupported public-key algorithm 1"));

        let v5 = mutated_secret_armor(|body| {
            body[0] = 5;
        });
        let err = parse_secret_signing_key(&v5, None)
            .expect_err("unsupported OpenPGP version is rejected");
        assert!(err.0.contains("only OpenPGP v4 public keys are supported"));
    }
}
