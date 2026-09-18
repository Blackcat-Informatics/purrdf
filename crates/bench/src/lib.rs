// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The deterministic scale-corpus generator.
//!
//! Capacity claims need a corpus a single trick cannot flatter: a generator
//! that mints only `…/e/{n}` hands the dictionary a front-coder's dream and
//! proves nothing. Every IRI here is minted **purely from its index** under a
//! fixed seed, drawn from five deliberately adversarial classes — front-codable
//! plain, long zero-padded numerics beyond machine integer widths, raw-Han
//! Chinese, host-scattered irregular, and very-long — so the corpus exercises
//! the same surfaces real mixed data does. Index-pure minting is also what
//! makes generation **shardable**: shard `k` of `n` emits exactly its slice of
//! the quad sequence, and the concatenation of all shards is byte-identical to
//! a single whole run.
//!
//! Everything is arithmetic on a [`splitmix64`] stream — no RNG syscalls, no
//! platform floats, no iteration-order dependence — so output is
//! byte-deterministic across targets, pinned by a golden digest test. The
//! library half is portable; I/O lives in the binary.
//!
//! This is corpus *generation* only. Output digests, ingest timings, and
//! capacity evidence are captured by the harnesses that consume it; the
//! statement layer and blob surfaces are out of scope for the scale corpus
//! (they are exercised by the envelope probe's keystone corpus).

use core::fmt::Write as _;

/// A corpus specification: everything the output bytes depend on.
///
/// Two specs that compare equal generate byte-identical corpora on every
/// target; the manifest digests exactly these fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpusSpec {
    /// Seed folded into every derivation.
    pub seed: u64,
    /// Total quads across all shards.
    pub quads: u64,
    /// Distinct-IRI target: entity IRIs are minted from indexes `0..iris`.
    pub iris: u64,
    /// This shard's zero-based index.
    pub shard: u64,
    /// Total shard count (`1` = whole corpus in one run).
    pub shards: u64,
}

impl CorpusSpec {
    /// The half-open quad-index range this shard emits.
    ///
    /// Shards partition `0..quads` contiguously; the last shard absorbs the
    /// remainder, and every boundary is a pure function of the spec.
    #[must_use]
    pub const fn shard_range(&self) -> (u64, u64) {
        let base = self.quads / self.shards;
        let start = self.shard * base;
        let end = if self.shard + 1 == self.shards {
            self.quads
        } else {
            start + base
        };
        (start, end)
    }
}

/// The stable identity of the default class mix. Any change to the mix, the
/// class shapes, or the row derivation is a new profile id — the manifest
/// carries it so a capture names exactly what generated its bytes.
pub const CORPUS_PROFILE_ID: &str = "purrdf-scale-mixed-v1";

/// The five IRI classes and their per-mille shares of the entity space.
/// The shares are the anti-compressibility contract: no single dictionary
/// trick can carry the whole corpus.
pub const CLASS_MIX_PER_MILLE: [(&str, u16); 5] = [
    ("plain", 400),
    ("numeric-long", 200),
    ("chinese", 200),
    ("irregular", 150),
    ("very-long", 50),
];

/// `splitmix64` — the classic public-domain mixing step: deterministic,
/// allocation-free, and identical on every target.
#[must_use]
pub const fn splitmix64(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Mixes the seed with a stream tag and an index into one draw.
const fn draw(seed: u64, tag: u64, index: u64) -> u64 {
    splitmix64(seed ^ splitmix64(tag ^ splitmix64(index)))
}

/// Which class an entity index belongs to (pure function of the index).
fn class_of(seed: u64, entity: u64) -> usize {
    let roll = draw(seed, 0xC1A5_5000, entity) % 1000;
    let mut acc = 0u64;
    for (position, (_, share)) in CLASS_MIX_PER_MILLE.iter().enumerate() {
        acc += u64::from(*share);
        if roll < acc {
            return position;
        }
    }
    CLASS_MIX_PER_MILLE.len() - 1
}

/// Appends entity IRI `entity`'s text (without angle brackets) to `out`.
///
/// Minting is index-pure: the same `(seed, entity)` always yields the same
/// IRI on every shard and every target.
pub fn write_entity_iri(out: &mut String, seed: u64, entity: u64) {
    let h = draw(seed, 0x1121_1121, entity);
    match class_of(seed, entity) {
        // Front-codable: the dictionary-friendly floor of the mix.
        0 => {
            let _ = write!(out, "https://example.org/e/{entity}");
        }
        // Long zero-padded numerics: 36 digits, beyond u64 and 2^53, with
        // leading zeros that MUST survive verbatim (identity, not number).
        1 => {
            let _ = write!(
                out,
                "https://example.org/n/{:018}{:018}",
                h % 1_000_000_000_000_000_000,
                entity % 1_000_000_000_000_000_000
            );
        }
        // Raw-Han path segments: IRIs carry non-ASCII directly (RFC 3987
        // `ucschar`), which N-Quads IRIREF admits raw — no percent-encoding.
        2 => {
            out.push_str("https://example.org/中文/");
            let mut v = h | 1;
            for _ in 0..4 {
                let cp = 0x4E00 + u32::try_from(v % 0x51A5).unwrap_or(0);
                out.push(char::from_u32(cp).unwrap_or('\u{4E00}'));
                v = splitmix64(v);
            }
            let _ = write!(out, "/{entity}");
        }
        // Host-scattered irregular: a hashed subdomain defeats host-prefix
        // sharing, and the mixed-case percent-encoded tail defeats suffix
        // tricks.
        3 => {
            let _ = write!(
                out,
                "https://s{:04x}.example.org/x/%{:02X}{:02X}/K{}~{:x}",
                h & 0xFFFF,
                0x41 + (h >> 16) % 26,
                0x61 + (h >> 24) % 26,
                entity,
                h
            );
        }
        // Very long: ~512 bytes of index-derived segments; length itself is
        // the stress (arena growth, bucket boundaries, wire framing).
        _ => {
            out.push_str("https://example.org/long");
            let mut v = h | 1;
            for _ in 0..30 {
                let _ = write!(out, "/seg{v:016x}");
                v = splitmix64(v);
            }
            let _ = write!(out, "/{entity}");
        }
    }
}

/// Skew mapping: quad slots draw entities with a hot head and a long tail
/// (integer approximation of a power-law; exact, no floats).
fn skewed_entity(seed: u64, tag: u64, slot: u64, iris: u64) -> u64 {
    let r = draw(seed, tag, slot);
    // Square the unit draw in fixed point: u^2 biases toward 0.
    let hi = r >> 32;
    let biased = (hi * hi) >> 32;
    (biased * iris) >> 32
}

/// The predicate vocabulary (small and fixed, as real datasets have).
const PREDICATES: [&str; 8] = [
    "https://example.org/p/rel",
    "https://example.org/p/name",
    "https://example.org/p/type",
    "https://example.org/p/part",
    "https://example.org/p/near",
    "https://example.org/p/note",
    "https://example.org/p/标签",
    "https://example.org/p/seen",
];

/// Appends quad `slot`'s N-Quads row (with trailing newline) to `out`.
///
/// Row shape: two thirds entity–entity edges, one third literals (plain
/// ASCII, `@zh` Han, or long text), a sixth of rows in one of 16 named
/// graphs — enough graph spread to exercise the quad position without
/// dominating the dictionary.
pub fn write_row(out: &mut String, spec: &CorpusSpec, slot: u64) {
    let seed = spec.seed;
    out.push('<');
    write_entity_iri(out, seed, skewed_entity(seed, 0x5AB1, slot, spec.iris));
    out.push_str("> <");
    out.push_str(PREDICATES[usize::try_from(draw(seed, 0x9AED, slot) % 8).unwrap_or(0)]);
    out.push_str("> ");
    let o = draw(seed, 0x0B1E, slot);
    match o % 3 {
        0 | 1 if !o.is_multiple_of(7) => {
            out.push('<');
            write_entity_iri(out, seed, skewed_entity(seed, 0x0B1F, slot, spec.iris));
            out.push('>');
        }
        _ => match o % 5 {
            0 => {
                let _ = write!(out, "\"value {}\"", o >> 8);
            }
            1 => {
                out.push('"');
                let mut v = o | 1;
                for _ in 0..3 {
                    let cp = 0x4E00 + u32::try_from(v % 0x51A5).unwrap_or(0);
                    out.push(char::from_u32(cp).unwrap_or('\u{4E00}'));
                    v = splitmix64(v);
                }
                out.push_str("\"@zh");
            }
            _ => {
                let _ = write!(out, "\"text {:064x} {:064x}\"", o, splitmix64(o));
            }
        },
    }
    if o.is_multiple_of(6) {
        let _ = write!(out, " <https://example.org/g/{}>", (o >> 16) % 16);
    }
    out.push_str(" .\n");
}

/// The manifest: everything a capture needs to name this corpus exactly.
#[must_use]
pub fn manifest(spec: &CorpusSpec) -> String {
    let (start, end) = spec.shard_range();
    let mut m = String::new();
    let _ = write!(
        m,
        "{{\"profile\": \"{CORPUS_PROFILE_ID}\", \"seed\": {}, \"quads\": {}, \"iris\": {}, \
         \"shard\": {}, \"shards\": {}, \"shard_rows\": [{start}, {end}], \"class_mix_per_mille\": {{",
        spec.seed, spec.quads, spec.iris, spec.shard, spec.shards
    );
    for (position, (name, share)) in CLASS_MIX_PER_MILLE.iter().enumerate() {
        let comma = if position + 1 == CLASS_MIX_PER_MILLE.len() {
            ""
        } else {
            ", "
        };
        let _ = write!(m, "\"{name}\": {share}{comma}");
    }
    m.push_str("}}\n");
    m
}

#[cfg(test)]
mod tests {
    use super::{CORPUS_PROFILE_ID, CorpusSpec, class_of, write_entity_iri, write_row};

    const SPEC: CorpusSpec = CorpusSpec {
        seed: 0x5EED_CAFE,
        quads: 2_000,
        iris: 1_000,
        shard: 0,
        shards: 1,
    };

    fn corpus(spec: &CorpusSpec) -> String {
        let (start, end) = spec.shard_range();
        let mut out = String::new();
        for slot in start..end {
            write_row(&mut out, spec, slot);
        }
        out
    }

    #[test]
    fn generation_is_deterministic_and_shard_concat_equals_whole() {
        let whole = corpus(&SPEC);
        assert_eq!(whole, corpus(&SPEC), "two runs must be byte-identical");
        let mut stitched = String::new();
        for shard in 0..3 {
            stitched.push_str(&corpus(&CorpusSpec {
                shard,
                shards: 3,
                ..SPEC
            }));
        }
        assert_eq!(whole, stitched, "shard concatenation must equal one run");
    }

    #[test]
    fn every_row_parses_as_strict_nquads() {
        let text = corpus(&SPEC);
        let dataset = purrdf_rdf::parse_dataset(text.as_bytes(), "application/n-quads", None)
            .expect("generated corpus must satisfy the strict reader");
        assert!(dataset.rdf_row_count() > 0);
    }

    #[test]
    fn class_mix_is_exercised_and_indexed_minting_is_stable() {
        let mut seen = [false; 5];
        for entity in 0..2_000 {
            seen[class_of(SPEC.seed, entity)] = true;
        }
        assert_eq!(seen, [true; 5], "every class must appear");
        let mut a = String::new();
        let mut b = String::new();
        write_entity_iri(&mut a, SPEC.seed, 42);
        write_entity_iri(&mut b, SPEC.seed, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn golden_digest_pins_the_profile() {
        // A change to any derivation is a NEW corpus profile: bump
        // CORPUS_PROFILE_ID and re-pin, never silently regenerate.
        let text = corpus(&SPEC);
        assert_eq!(
            text.lines().count(),
            2_000,
            "emitted line count is exact (the dataset itself may hold fewer:
             identical rows deduplicate under set semantics)"
        );
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for byte in text.bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3);
        }
        assert_eq!(
            hash, 0xB6E8_8723_8FD8_AB29,
            "byte-level FNV pin moved: {CORPUS_PROFILE_ID} must be bumped"
        );
    }
}
