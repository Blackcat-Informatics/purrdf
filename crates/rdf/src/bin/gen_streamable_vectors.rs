// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Re-freeze the streamable-compaction corpus vector (GTS streamable
//! compaction + certification): `vectors/25b-streamable-compacted.gts` and
//! its companion `vectors/25b-streamable-compacted.expected.json` fold.
//!
//! Maintainer-only binary, mirroring `gen_dict_vectors`: `25b-streamable-compacted`
//! has always meant "the streamable compaction of `vectors/25-streamable-source.gts`".
//! This binary reads the UNTOUCHED, already-frozen `25-streamable-source.gts` from
//! disk (it is never regenerated or hand-edited here), compacts it through the
//! CURRENT production compactor
//! (`purrdf_gts::compact::compact_streamable` via
//! `purrdf_rdf::gts_certify::compact_and_certify`, so the frozen pack also carries
//! a mandatory packaging signature and the carried-forward source authorship
//! signatures stay bound under `stream:detachedSignatureRoot`), and writes the
//! resulting pack plus its expected-fold JSON.
//!
//! ## Dictionary strategy
//!
//! `25-streamable-source.gts` carries exactly two content blobs, 10 and 100
//! decoded bytes — nowhere near enough repeated structure for FastCOVER to train
//! a meaningful dictionary against (see `crates/gts/src/dict.rs`), and not the
//! purpose of this vector (that is what `30-dict-rawcontent`/`31-dict-trained`
//! freeze). This vector freezes streamable-compaction + certification in
//! isolation, so it pins [`DictPlan::undicted`] deliberately: no `"dct"` header
//! entry, plain (undicted) `zstd`/`identity` frames, and no new
//! `zstd`/`dct`-capability requirement on the manifest entry.
//!
//! ## Fold JSON
//!
//! `vectors/25b-streamable-compacted.expected.json` is regenerated from the
//! FRESH pack's fold (`purrdf_gts::reader::read`), never hand-typed: term/quad
//! counts straight off the folded [`purrdf_gts::model::Graph`], the N-Quads text
//! rendered through the same GTS→dataset bridge and native N-Quads serializer
//! `purrdf_rdf::gts_certify` verification uses, sorted lexicographically, and
//! written with the same `sort_keys=true`, one-space-indent JSON style as every
//! other `vectors/*.expected.json` in this corpus.
//!
//! Re-running this binary regenerates both files byte-identically: `zstd`/plain
//! frame compaction is a pure function of the source bytes and the fixed
//! timestamp/keys below, with no platform-dependent floating point on this path
//! (unlike `31-dict-trained`'s FastCOVER training).

use std::path::Path;

use ed25519_dalek::SigningKey;
use purrdf_gts::compact::DictPlan;
use purrdf_rdf::capture_support::corpus_repo_root;
use purrdf_rdf::gts_certify::compact_and_certify;
use purrdf_rdf::gts_dict_vectors::{expected_fold_json, render_expected_json};

/// The rewrite time recorded as `stream:timestamp` — matches
/// `gen_dict_vectors::TIMESTAMP` so every frozen corpus vector authored under
/// this task series shares one fixed authoring instant.
const TIMESTAMP: &str = "2026-01-01T00:00:00Z";

/// The fixed packaging signing key (`kid` "pack") `25b-streamable-compacted.gts`
/// is packaged with — the MANDATORY streamable-compaction ordering/packaging
/// signature (GTS-SPEC §10.1), never frame authorship (the source's own
/// authorship signatures, signed under whatever key `25-streamable-source.gts`
/// carries, ride through untouched as carried-forward detached-signature
/// provenance). Deliberately a DIFFERENT key from `gen_dict_vectors`'
/// `packaging_key` ([3u8; 32]/[7u8; 32]): distinct frozen corpora should not
/// share signing key material even when both are fixed maintainer constants.
fn packaging_key() -> SigningKey {
    SigningKey::from_bytes(&[11u8; 32])
}

fn vectors_dir() -> std::path::PathBuf {
    corpus_repo_root().join("vectors")
}

/// `vectors/25-streamable-source.gts`'s current bytes, read verbatim.
///
/// This source vector is NEVER regenerated here — it is a separately frozen,
/// hand-curated top-level GTS vector (`25-streamable-source`) that this binary
/// only reads.
fn read_source() -> Vec<u8> {
    let path = vectors_dir().join("25-streamable-source.gts");
    std::fs::read(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn write_vector(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
    println!("wrote {} ({} bytes)", path.display(), bytes.len());
}

fn main() {
    let source = read_source();
    let vectors_dir = vectors_dir();

    let (pack, _cert) = compact_and_certify(
        &source,
        DictPlan::undicted(),
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("streamable compaction over the frozen 25-streamable-source succeeds");
    write_vector(&vectors_dir.join("25b-streamable-compacted.gts"), &pack);

    let expected = expected_fold_json(&pack);
    let rendered = render_expected_json(&expected);
    std::fs::write(
        vectors_dir.join("25b-streamable-compacted.expected.json"),
        &rendered,
    )
    .expect("write 25b-streamable-compacted.expected.json");
    println!(
        "wrote {} ({} bytes)",
        vectors_dir
            .join("25b-streamable-compacted.expected.json")
            .display(),
        rendered.len()
    );
}
