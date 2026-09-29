// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Published known answers on every execution path the host runs, streaming
//! at every split point, and dispatch through `&mut dyn Digest`.
//!
//! Sources of the expected values:
//! * MD5: RFC 1321 §A.5, "test suite".
//! * SHA-1: the NIST example values for "abc" and the 448-bit two-block
//!   message, and FIPS 180-2 Appendix A.3 for one million "a".
//! * SHA-3: the NIST FIPS 202 example values for the empty message and the
//!   1600-bit message of 200 bytes 0xA3.
//! * CRC-32/ISO-HDLC: its check value, the CRC of "123456789".

use purrdf_hash::backend::{Crc32Backend, Sha1Backend};
use purrdf_hash::crc32::Crc32;
use purrdf_hash::md5::Md5;
use purrdf_hash::sha1::Sha1;
use purrdf_hash::sha3::{Sha3_224, Sha3_256, Sha3_384, Sha3_512};
use purrdf_hash::{Backend as _, Digest, MAX_OUTPUT_LEN};

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// The helper renders every length 0..=64 from every first byte exactly as
/// the crate's base16 encoder does.
#[test]
fn hex_helper_matches_the_crate_encoder() {
    for len in 0..=64_usize {
        for first in 0..=255_u8 {
            let input: Vec<u8> = (0..len).map(|i| first.wrapping_add(i as u8)).collect();
            assert_eq!(hex(&input), purrdf_hash::hex::Lower(&input).to_string());
        }
    }
}

#[test]
fn kat_md5_rfc1321_test_suite() {
    let suite: [(&str, &str); 7] = [
        ("", "d41d8cd98f00b204e9800998ecf8427e"),
        ("a", "0cc175b9c0f1b6a831c399e269772661"),
        ("abc", "900150983cd24fb0d6963f7d28e17f72"),
        ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
        (
            "abcdefghijklmnopqrstuvwxyz",
            "c3fcd3d76192e4007dfb496cca67e13b",
        ),
        (
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
            "d174ab98d277d9f5a5611c2c9f419d9f",
        ),
        (
            "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
            "57edf4a22be3c955ac49da2e2107b67a",
        ),
    ];
    for (message, expected) in suite {
        assert_eq!(
            hex(&Md5::digest(message.as_bytes())),
            expected,
            "MD5({message:?})"
        );
    }
}

#[test]
fn kat_sha1_nist_examples_on_every_path() {
    let examples = [
        ("abc", "a9993e364706816aba3e25717850c26c9cd0d89d"),
        (
            "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
        ),
    ];
    for (message, expected) in examples {
        assert_eq!(hex(&Sha1::digest(message.as_bytes())), expected);
        for backend in Sha1Backend::all_available() {
            let digest = backend.digest(message.as_bytes()).expect("available");
            assert_eq!(hex(&digest), expected, "{} on {message:?}", backend.name());
        }
    }
}

#[test]
fn million_a_sha1_on_every_path() {
    let message = vec![b'a'; 1_000_000];
    for backend in Sha1Backend::all_available() {
        let digest = backend.digest(&message).expect("available");
        assert_eq!(
            hex(&digest),
            "34aa973cd4c4daa4f61eeb2bdbad27316534016f",
            "{}",
            backend.name()
        );
    }
}

#[test]
fn kat_sha3_nist_examples() {
    assert_eq!(
        hex(&Sha3_224::digest(b"")),
        "6b4e03423667dbb73b6e15454f0eb1abd4597f9a1b078e3f5b5a6bc7"
    );
    assert_eq!(
        hex(&Sha3_256::digest(b"")),
        "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
    );
    assert_eq!(
        hex(&Sha3_384::digest(b"")),
        "0c63a75b845e4f7d01107d852e4c2485c51a50aaaa94fc61995e71bbee983a2a\
         c3713831264adb47fb6bd1e058d5f004"
    );
    assert_eq!(
        hex(&Sha3_512::digest(b"")),
        "a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a6\
         15b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26"
    );
    let message = [0xA3u8; 200];
    assert_eq!(
        hex(&Sha3_224::digest(&message)),
        "9376816aba503f72f96ce7eb65ac095deee3be4bf9bbc2a1cb7e11e0"
    );
    assert_eq!(
        hex(&Sha3_256::digest(&message)),
        "79f38adec5c20307a98ef76e8324afbfd46cfd81b22e3973c65fa1bd9de31787"
    );
    assert_eq!(
        hex(&Sha3_384::digest(&message)),
        "1881de2ca7e41ef95dc4732b8f5f002b189cc1e42b74168ed1732649ce1dbcdd\
         76197a31fd55ee989f2d7050dd473e8f"
    );
    assert_eq!(
        hex(&Sha3_512::digest(&message)),
        "e76dfad22084a8b1467fcf2ffa58361bec7628edf5f3fdc0e4805dc48caeeca8\
         1b7c13c30adf52a3659584739a2df46be589c51ca1a4a8416df6545a1ce8ba00"
    );
}

#[test]
fn kat_crc32_check_value_on_every_path() {
    assert_eq!(Crc32::checksum(b"123456789"), 0xCBF4_3926);
    for backend in Crc32Backend::all_available() {
        assert_eq!(
            backend.checksum(b"123456789"),
            Some(0xCBF4_3926),
            "{}",
            backend.name()
        );
        assert_eq!(backend.checksum(b""), Some(0), "{}", backend.name());
    }
}

/// 300 bytes with no period short enough to hide a buffering error.
fn split_input() -> Vec<u8> {
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    (0..300)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            (state >> 56) as u8
        })
        .collect()
}

/// Every split of the input into two `update` calls gives the one-shot digest.
fn assert_every_split(name: &str, hasher: &mut dyn Digest, one_shot: &[u8]) {
    let data = split_input();
    for split in 0..=data.len() {
        hasher.reset();
        hasher.update(&data[..split]);
        hasher.update(&data[split..]);
        let mut out = [0u8; MAX_OUTPUT_LEN];
        let len = hasher.finalize_reset(&mut out);
        assert_eq!(&out[..len], one_shot, "{name} split at {split}");
    }
}

#[test]
fn split_md5_every_point() {
    let data = split_input();
    assert_every_split("md5", &mut Md5::new(), &Md5::digest(&data));
}

#[test]
fn split_sha1_every_point_on_every_path() {
    let data = split_input();
    let expected = Sha1Backend::Portable
        .digest(&data)
        .expect("always available");
    for backend in Sha1Backend::all_available() {
        let mut hasher = backend.hasher().expect("available");
        assert_every_split(backend.name(), &mut hasher, &expected);
    }
}

#[test]
fn split_sha3_every_point() {
    let data = split_input();
    assert_every_split("sha3-224", &mut Sha3_224::new(), &Sha3_224::digest(&data));
    assert_every_split("sha3-256", &mut Sha3_256::new(), &Sha3_256::digest(&data));
    assert_every_split("sha3-384", &mut Sha3_384::new(), &Sha3_384::digest(&data));
    assert_every_split("sha3-512", &mut Sha3_512::new(), &Sha3_512::digest(&data));
}

#[test]
fn split_crc32_every_point_on_every_path() {
    let data = split_input();
    let expected = Crc32Backend::Portable
        .checksum(&data)
        .expect("always available")
        .to_be_bytes();
    for backend in Crc32Backend::all_available() {
        let mut hasher = backend.hasher().expect("available");
        assert_every_split(backend.name(), &mut hasher, &expected);
    }
}

/// The dispatch a SPARQL hash built-in can make: the function picks the
/// hasher, the lexical form is fed through `&mut dyn Digest`, and the answer
/// is the inherent one-shot's.
#[test]
fn kat_dispatch_through_dyn_digest() {
    fn hash_with(hasher: &mut dyn Digest, lexical: &str) -> String {
        hasher.update(lexical.as_bytes());
        let mut out = [0u8; MAX_OUTPUT_LEN];
        let len = hasher.finalize_reset(&mut out);
        assert_eq!(len, hasher.output_len());
        hex(&out[..len])
    }
    let lexical = "abc";
    let mut md5 = Md5::new();
    let mut sha1 = Sha1::new();
    let mut sha3_224 = Sha3_224::new();
    let mut sha3_256 = Sha3_256::new();
    let mut sha3_384 = Sha3_384::new();
    let mut sha3_512 = Sha3_512::new();
    let cases: [(&mut dyn Digest, String); 6] = [
        (&mut md5, hex(&Md5::digest(lexical.as_bytes()))),
        (&mut sha1, hex(&Sha1::digest(lexical.as_bytes()))),
        (&mut sha3_224, hex(&Sha3_224::digest(lexical.as_bytes()))),
        (&mut sha3_256, hex(&Sha3_256::digest(lexical.as_bytes()))),
        (&mut sha3_384, hex(&Sha3_384::digest(lexical.as_bytes()))),
        (&mut sha3_512, hex(&Sha3_512::digest(lexical.as_bytes()))),
    ];
    for (hasher, expected) in cases {
        // Twice: `finalize_reset` must leave the hasher as new.
        assert_eq!(hash_with(hasher, lexical), expected);
        assert_eq!(hash_with(hasher, lexical), expected);
    }
    assert_eq!(
        hex(&Md5::digest(b"abc")),
        "900150983cd24fb0d6963f7d28e17f72"
    );
}
