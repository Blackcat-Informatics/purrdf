// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//! Paired public-API comparison; removed packages exist only in this external harness.

use md5::Digest as _;
use purrdf_hash::fixed::FixedHasher;
use std::{
    collections::HashMap,
    hash::{BuildHasherDefault, Hash, Hasher},
    hint::black_box,
    time::Instant,
};
const N: usize = 1024;

fn data(len: usize, seed: u64) -> Vec<u8> {
    let mut x = seed | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect()
}
fn time<F: FnMut() -> u64>(f: &mut F, rounds: u64) -> f64 {
    let now = Instant::now();
    for _ in 0..rounds {
        black_box(f());
    }
    now.elapsed().as_nanos() as f64 / rounds as f64
}
fn pair<A: FnMut() -> u64, B: FnMut() -> u64>(
    name: &str,
    bytes: usize,
    units: usize,
    mut a: A,
    mut b: B,
) {
    black_box(a());
    black_box(b());
    let mut rounds = 1u64;
    while time(&mut a, rounds) * (rounds as f64) < 15_000_000.0 {
        rounds *= 2;
    }
    time(&mut b, rounds);
    for sample in 0..12 {
        let (old, new) = if sample % 2 == 0 {
            (time(&mut a, rounds), time(&mut b, rounds))
        } else {
            let new = time(&mut b, rounds);
            (time(&mut a, rounds), new)
        };
        println!(
            "{name},{bytes},{units},{sample},{rounds},{:.6},{:.6}",
            old / units as f64,
            new / units as f64
        );
    }
}
fn keys<H: Hasher + Default>(
    kind: &str,
    strings: &[String],
    bytes: &[Vec<u8>],
    native: bool,
) -> u64 {
    let mut sum = 0u64;
    for i in 0..N {
        let n = black_box((i as u64).wrapping_mul(0x9e3779b97f4a7c15));
        let text = black_box(strings[i].as_str());
        if kind == "iri" && native {
            sum ^= black_box(FixedHasher::hash_terminal(0, text.as_bytes()));
            continue;
        }
        if native && ["blank", "literal", "language"].contains(&kind) {
            let mut h = FixedHasher::default();
            if kind == "blank" {
                h.write_u64(1u64 << 32);
                h.write(text.as_bytes());
            } else {
                let metadata = 2u128 | (u128::from(kind == "language") << 2);
                h.write_u128(17 | (metadata << 64));
                h.write(text.as_bytes());
                if kind == "language" {
                    h.write(b"en");
                }
            }
            sum ^= black_box(h.finish());
            continue;
        }
        let mut h = H::default();
        match kind {
            "u32" => h.write_u32(n as u32),
            "u64" => h.write_u64(n),
            "triple" => {
                3u8.hash(&mut h);
                (n as u32).hash(&mut h);
                ((n >> 16) as u32).hash(&mut h);
                ((n >> 32) as u32).hash(&mut h);
            }
            "blank" => {
                1u8.hash(&mut h);
                text.hash(&mut h);
                0u64.hash(&mut h);
            }
            "literal" | "language" => {
                2u8.hash(&mut h);
                text.hash(&mut h);
                17u32.hash(&mut h);
                let lang = if kind == "language" { Some("en") } else { None };
                lang.hash(&mut h);
                Option::<u8>::None.hash(&mut h);
            }
            "iri" => {
                0u8.hash(&mut h);
                text.hash(&mut h);
            }
            "str" => text.hash(&mut h),
            "bytes" => h.write(black_box(&bytes[i])),
            _ => unreachable!(),
        }
        sum ^= black_box(h.finish());
    }
    sum
}
fn maps<H: Hasher + Default>(strings: &[String], lookup: bool) -> u64 {
    let mut map: HashMap<&str, u64, BuildHasherDefault<H>> =
        HashMap::with_capacity_and_hasher(N, Default::default());
    for (i, s) in strings.iter().enumerate() {
        map.insert(black_box(s), i as u64);
    }
    if lookup {
        strings
            .iter()
            .map(|s| *black_box(map.get(black_box(s.as_str())).unwrap()))
            .sum()
    } else {
        black_box(map.len()) as u64
    }
}
fn digest_word(bytes: impl AsRef<[u8]>) -> u64 {
    let value = black_box(bytes);
    let mut out = [0; 8];
    let n = value.as_ref().len().min(8);
    out[..n].copy_from_slice(&value.as_ref()[..n]);
    u64::from_le_bytes(out)
}
fn main() {
    let filter = std::env::args().nth(1).unwrap_or_default();
    eprintln!(
        "target AES: {}; SHA1: {}; CRC32: {}",
        cfg!(target_feature = "aes"),
        purrdf_hash::backend::Sha1Backend::selected().name(),
        purrdf_hash::backend::Crc32Backend::selected().name()
    );
    println!("case,bytes,units,sample,rounds,baseline_ns,candidate_ns");
    if filter.is_empty() || filter == "table" {
        for (kind, len) in [
            ("u32", 4usize),
            ("u64", 8),
            ("triple", 12),
            ("blank", 16),
            ("literal", 16),
            ("literal", 256),
            ("language", 32),
            ("str", 26),
            ("str", 256),
            ("iri", 26),
            ("iri", 256),
            ("bytes", 0),
            ("bytes", 8),
            ("bytes", 16),
            ("bytes", 32),
            ("bytes", 64),
            ("bytes", 128),
            ("bytes", 1024),
            ("bytes", 16384),
        ] {
            let strings: Vec<String> = (0..N)
                .map(|i| {
                    let prefix = if kind == "iri" {
                        format!("http://example.org/{i:04}/")
                    } else {
                        format!("{i:08}")
                    };
                    format!("{prefix}{}", "x".repeat(len.saturating_sub(prefix.len())))
                        .chars()
                        .take(len)
                        .collect()
                })
                .collect();
            let bytes: Vec<Vec<u8>> = (0..N).map(|i| data(len, i as u64 + 7)).collect();
            pair(
                &format!("table/{kind}"),
                len,
                N,
                || keys::<ahash::AHasher>(kind, &strings, &bytes, false),
                || keys::<FixedHasher>(kind, &strings, &bytes, true),
            );
        }
        let strings: Vec<String> = (0..N).map(|i| format!("http://example.org/{i}")).collect();
        for lookup in [false, true] {
            pair(
                if lookup {
                    "map/insert+lookup-str"
                } else {
                    "map/insert-str"
                },
                0,
                N,
                || maps::<ahash::AHasher>(&strings, lookup),
                || maps::<FixedHasher>(&strings, lookup),
            );
        }
    }
    if filter.is_empty() || filter == "digests" {
        for len in [0, 16, 64, 1024, 65536, 1048576] {
            let bytes = data(len, 123);
            macro_rules! digest_pair {
                ($label:expr,$old:ty,$new:ty) => {{
                    assert_eq!(
                        <$old>::digest(&bytes).as_slice(),
                        <$new>::digest(&bytes).as_slice()
                    );
                    pair(
                        $label,
                        len,
                        1,
                        || digest_word(<$old>::digest(black_box(&bytes))),
                        || digest_word(<$new>::digest(black_box(&bytes))),
                    );
                }};
            }
            digest_pair!("digest/md5", md5::Md5, purrdf_hash::md5::Md5);
            digest_pair!("digest/sha1", sha1::Sha1, purrdf_hash::sha1::Sha1);
            digest_pair!(
                "digest/sha3-224",
                sha3::Sha3_224,
                purrdf_hash::sha3::Sha3_224
            );
            digest_pair!(
                "digest/sha3-256",
                sha3::Sha3_256,
                purrdf_hash::sha3::Sha3_256
            );
            digest_pair!(
                "digest/sha3-384",
                sha3::Sha3_384,
                purrdf_hash::sha3::Sha3_384
            );
            digest_pair!(
                "digest/sha3-512",
                sha3::Sha3_512,
                purrdf_hash::sha3::Sha3_512
            );
            assert_eq!(
                crc32fast::hash(&bytes),
                purrdf_hash::crc32::Crc32::checksum(&bytes)
            );
            pair(
                "digest/crc32",
                len,
                1,
                || u64::from(crc32fast::hash(black_box(&bytes))),
                || u64::from(purrdf_hash::crc32::Crc32::checksum(black_box(&bytes))),
            );
            pair(
                "control/blake3",
                len,
                1,
                || digest_word(*blake3::hash(black_box(&bytes)).as_bytes()),
                || digest_word(*blake3::hash(black_box(&bytes)).as_bytes()),
            );
        }
    }
    if let Some(name) = filter.strip_prefix("blake3-backend-") {
        use purrdf_hash::blake3::Backend;
        let backend = match name {
            "portable" => Backend::Portable,
            "sse2" => Backend::Sse2,
            "ssse3" => Backend::Ssse3,
            "avx2" => Backend::Avx2,
            "avx512" => Backend::Avx512,
            _ => panic!("backend"),
        };
        assert!(backend.is_available());
        for len in [0usize, 64, 1024, 4096, 16384, 65536, 1048576] {
            let bytes = data(len, 123);
            assert_eq!(
                backend.hash(&bytes).unwrap().as_bytes(),
                blake3::hash(&bytes).as_bytes()
            );
            pair(
                &format!("pinned/{name}/one-shot"),
                len,
                1,
                || digest_word(*blake3::hash(black_box(&bytes)).as_bytes()),
                || digest_word(*backend.hash(black_box(&bytes)).unwrap().as_bytes()),
            );
            if len == 64 || len == 1048576 {
                for width in [8, 64, 1024, 16384, 65536] {
                    if width > len {
                        continue;
                    }
                    pair(
                        &format!("pinned/{name}/stream-{width}"),
                        len,
                        1,
                        || {
                            let mut h = blake3::Hasher::new();
                            for part in black_box(&bytes).chunks(width) {
                                h.update(part);
                            }
                            digest_word(*h.finalize().as_bytes())
                        },
                        || {
                            let mut h = backend.hasher().unwrap();
                            for part in black_box(&bytes).chunks(width) {
                                h.update(part);
                            }
                            digest_word(*h.finalize().as_bytes())
                        },
                    );
                }
            }
        }
    }
    if filter == "blake3-stream-native" {
        for len in [64usize, 1024, 4096, 65536, 1048576] {
            let bytes = data(len, 123);
            for width in [1usize, 8, 64, 1024, 16384, 65536, usize::MAX] {
                if width != usize::MAX && width > len {
                    continue;
                }
                let chunks = || {
                    let bytes = bytes.as_slice();
                    let mut offset = 0usize;
                    let mut step = 0usize;
                    std::iter::from_fn(move || {
                        if offset == bytes.len() {
                            return None;
                        }
                        let count = if width == usize::MAX {
                            [1, 63, 1025, 7, 16384, 65, 8191][step % 7]
                        } else {
                            width
                        };
                        let end = (offset + count).min(bytes.len());
                        let part = &bytes[offset..end];
                        offset = end;
                        step += 1;
                        Some(part)
                    })
                };
                let old = || {
                    let mut h = blake3::Hasher::new();
                    for part in chunks() {
                        h.update(black_box(part));
                    }
                    *h.finalize().as_bytes()
                };
                let new = || {
                    let mut h = purrdf_hash::blake3::Hasher::new();
                    for part in chunks() {
                        h.update(black_box(part));
                    }
                    *h.finalize().as_bytes()
                };
                assert_eq!(old(), new());
                assert_eq!(&old(), blake3::hash(&bytes).as_bytes());
                let name = if width == usize::MAX {
                    "irregular".to_string()
                } else {
                    width.to_string()
                };
                pair(
                    &format!("blake3/native-stream-{name}"),
                    len,
                    1,
                    || digest_word(old()),
                    || digest_word(new()),
                );
            }
        }
    }
    if filter == "blake3-select" {
        for len in [0usize, 16, 64, 65, 1024, 4096, 16384, 65536, 1048576] {
            let bytes = data(len, 123);
            for backend in purrdf_hash::blake3::Backend::ALL {
                if !backend.is_available() {
                    continue;
                }
                pair(
                    &format!("select/{backend:?}"),
                    len,
                    1,
                    || digest_word(*purrdf_hash::blake3::hash(black_box(&bytes)).as_bytes()),
                    || digest_word(*backend.hash(black_box(&bytes)).unwrap().as_bytes()),
                );
            }
        }
    }
    if filter == "blake3-join" {
        struct Scheduler;
        impl purrdf_hash::blake3::Join for Scheduler {
            fn join<A: Send, B: Send>(
                &self,
                left: impl FnOnce() -> A + Send,
                right: impl FnOnce() -> B + Send,
            ) -> (A, B) {
                rayon::join(left, right)
            }
        }
        for len in [131072usize, 1048576, 16777216] {
            let bytes = data(len, 123);
            for grain in [32768usize, 65536, 131072, 262144, 1048576] {
                let old = || {
                    let mut h = blake3::Hasher::new();
                    h.update_rayon(black_box(&bytes));
                    *h.finalize().as_bytes()
                };
                let new = || {
                    *purrdf_hash::blake3::hash_with_join(black_box(&bytes), grain, &Scheduler)
                        .as_bytes()
                };
                assert_eq!(old(), new());
                pair(
                    &format!("blake3/join-{grain}"),
                    len,
                    1,
                    || digest_word(old()),
                    || digest_word(new()),
                );
                pair(
                    &format!("blake3/join-{grain}-vs-serial"),
                    len,
                    1,
                    || digest_word(*purrdf_hash::blake3::hash(black_box(&bytes)).as_bytes()),
                    || digest_word(new()),
                );
            }
        }
    }
    if filter == "blake3-native" {
        for len in [0usize, 16, 64, 1024, 4096, 65536, 131072, 1048576, 16777216] {
            let bytes = data(len, 123);
            assert_eq!(
                blake3::hash(&bytes).as_bytes(),
                purrdf_hash::blake3::hash(&bytes).as_bytes()
            );
            pair(
                "blake3/native",
                len,
                1,
                || digest_word(*blake3::hash(black_box(&bytes)).as_bytes()),
                || digest_word(*purrdf_hash::blake3::hash(black_box(&bytes)).as_bytes()),
            );
        }
    }
    if filter.is_empty() || filter == "blake3" {
        for len in [64, 1024, 4096, 65536, 131072, 1048576, 16777216] {
            let bytes = data(len, 123);
            for chunk in [8, 64, 1024, 65536] {
                if chunk > len {
                    continue;
                }
                let expected = blake3::hash(&bytes);
                let stream = || {
                    let mut h = blake3::Hasher::new();
                    for part in black_box(&bytes).chunks(chunk) {
                        h.update(part);
                    }
                    *h.finalize().as_bytes()
                };
                assert_eq!(expected.as_bytes(), &stream());
                pair(
                    &format!("blake3/stream-{chunk}"),
                    len,
                    1,
                    || digest_word(*blake3::hash(black_box(&bytes)).as_bytes()),
                    || digest_word(stream()),
                );
            }
            if len >= 65536 {
                pair(
                    "blake3/rayon",
                    len,
                    1,
                    || digest_word(*blake3::hash(black_box(&bytes)).as_bytes()),
                    || {
                        let mut h = blake3::Hasher::new();
                        h.update_rayon(black_box(&bytes));
                        digest_word(*h.finalize().as_bytes())
                    },
                );
            }
        }
    }
}
