// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Freeze the dict-compaction corpus vectors (Task 7, Part A).
//!
//! Maintainer-only binary, mirroring `capture_sparql_goldens`: it authors ONE
//! fixed, signed GTS source (a stable corpus of repeated-structure content
//! blobs — exactly the shape a pack dictionary has something to train on),
//! compacts it under both [`DictStrategy::RawContent`] and
//! [`DictStrategy::Trained`] via `purrdf_rdf::gts_certify::compact_and_certify`,
//! and writes the resulting packs as the frozen corpus vectors
//! `vectors/30-dict-rawcontent.gts` and `vectors/31-dict-trained.gts`.
//!
//! `compact_and_certify` is used (rather than the bare
//! `purrdf_gts::compact::compact_streamable`) so the source's detached
//! authorship signature is carried forward, bound under
//! `stream:detachedSignatureRoot`, and the pack itself carries a mandatory
//! packaging (index/head) signature — the frozen vectors exercise the WHOLE
//! streamable-compaction + in-band-dictionary feature, not just the codec.
//!
//! Re-running this binary regenerates `30-dict-rawcontent.gts` byte-identically
//! (the raw-content dict producer has no platform-dependent floating point);
//! `31-dict-trained.gts` is expected to reproduce on the SAME authoring
//! platform but MAY differ across platforms because FastCOVER's scoring
//! involves transcendental floating point (see `crates/gts/src/dict.rs`).
//! `crates/gts/tests/dict_vectors.rs` is the drift guard: byte-equality for
//! the raw-content vector, fold-equality for the trained vector.

use std::path::Path;

use ed25519_dalek::SigningKey;
use purrdf_gts::compact::DictStrategy;
use purrdf_gts::writer::Writer;
use purrdf_rdf::capture_support::corpus_repo_root;
use purrdf_rdf::gts_certify::compact_and_certify;

const TIMESTAMP: &str = "2026-01-01T00:00:00Z";

/// The fixed authorship signing key (`kid` "authorA") every dict-vector source
/// is signed with.
fn authorship_key() -> SigningKey {
    SigningKey::from_bytes(&[3u8; 32])
}

/// The fixed packaging signing key (`kid` "pack") every dict-vector pack is
/// packaged with.
fn packaging_key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

/// A fixed, signed GTS source: 40 content-blob frames of repeated structure
/// (a `<s{i%7}> <p> "dict vector claim {i} about cats"` N-Triples line per
/// blob), signed under the fixed authorship key, closed with an `index`
/// footer — a stable corpus a pack dictionary strategy has real structure to
/// train on.
fn fixed_source() -> Vec<u8> {
    let mut w = Writer::new("purrdf.gts");
    w.sign_with(authorship_key(), "authorA");
    for i in 0..40u32 {
        let blob = format!(
            "<https://example.org/s{}> <https://example.org/p> \"dict vector claim {} about cats\" .\n",
            i % 7,
            i
        )
        .into_bytes();
        w.add_blob_owned(blob, Some("text/plain"), None);
    }
    w.add_index();
    w.into_bytes()
}

fn write_vector(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
    println!("wrote {} ({} bytes)", path.display(), bytes.len());
}

fn main() {
    let source = fixed_source();
    let vectors_dir = corpus_repo_root().join("vectors");

    let (raw_pack, _raw_cert) = compact_and_certify(
        &source,
        DictStrategy::RawContent,
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("raw-content dict compaction succeeds over the fixed source");
    write_vector(&vectors_dir.join("30-dict-rawcontent.gts"), &raw_pack);

    let (trained_pack, _trained_cert) = compact_and_certify(
        &source,
        DictStrategy::Trained,
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("trained dict compaction succeeds over the fixed source");
    write_vector(&vectors_dir.join("31-dict-trained.gts"), &trained_pack);
}
