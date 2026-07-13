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
//! isolation, so it pins [`DictStrategy::None`] deliberately: no `"dct"` header
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

use std::collections::BTreeMap;
use std::path::Path;

use ed25519_dalek::SigningKey;
use purrdf_gts::compact::DictStrategy;
use purrdf_gts::reader::read;
use purrdf_gts::wire::{hex, map_get};
use purrdf_rdf::capture_support::corpus_repo_root;
use purrdf_rdf::gts::dataset_from_gts_graph;
use purrdf_rdf::gts_certify::compact_and_certify;
use purrdf_rdf::{SerializeGraph, serialize_dataset};
use serde_json::{Value as Json, json};

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

/// The blob table as `digest -> {"mt": ..., "size": ...}`, `mt` from the
/// folded blob's declared `"pub"` metadata and `size` its DECODED length
/// (matching every other `vectors/*.expected.json` blob entry).
fn blobs_json(g: &purrdf_gts::model::Graph) -> Json {
    let mut out = BTreeMap::new();
    for (digest, entry) in &g.blobs {
        let mt = g
            .blob_meta
            .iter()
            .find(|(d, _)| d == digest)
            .and_then(|(_, meta)| match meta {
                ciborium::value::Value::Map(entries) => match map_get(entries, "mt") {
                    Some(ciborium::value::Value::Text(t)) => Some(t.clone()),
                    _ => None,
                },
                _ => None,
            })
            .unwrap_or_else(|| panic!("blob {digest} has no declared \"mt\" metadata"));
        let size = entry
            .decoded_len()
            .unwrap_or_else(|err| panic!("blob {digest} decodes: {err}"));
        out.insert(digest.clone(), json!({"mt": mt, "size": size}));
    }
    Json::Object(out.into_iter().collect())
}

/// The folded pack's content, rendered as sorted N-Quads text lines through
/// the SAME GTS→dataset bridge and native N-Quads serializer
/// `purrdf_rdf::gts_certify` verification uses — never a bespoke renderer that
/// could silently drift from the production fold path.
fn nquads_sorted(g: &purrdf_gts::model::Graph) -> Vec<String> {
    let dataset =
        dataset_from_gts_graph(g).unwrap_or_else(|err| panic!("GTS to dataset bridge: {err}"));
    let text = serialize_dataset(&dataset, "application/n-quads", SerializeGraph::Dataset)
        .unwrap_or_else(|err| panic!("serialize n-quads: {err:?}"));
    let text = String::from_utf8(text).expect("n-quads serializer emits UTF-8");
    let mut lines: Vec<String> = text
        .lines()
        .map(str::to_owned)
        .filter(|l| !l.is_empty())
        .collect();
    lines.sort();
    lines
}

/// The expected-fold JSON for `pack`, built entirely from its OWN fresh fold —
/// matching the schema (`docs/GTS-CONFORMANCE.md` §4) and `sort_keys=true`,
/// one-space-indent style of every other `vectors/*.expected.json`.
fn expected_fold_json(pack: &[u8]) -> Json {
    let g = read(pack, true, None);
    assert!(
        g.diagnostics.is_empty(),
        "the freshly compacted pack must fold cleanly: {:?}",
        g.diagnostics
    );

    let nquads = nquads_sorted(&g);
    let mut profiles: Vec<String> = g.segment_profiles.clone();
    profiles.sort();
    profiles.dedup();
    let streamable: Vec<Json> = g
        .segment_streamable
        .iter()
        .map(|s| json!({"claimed": s.claimed, "covered": s.covered, "tail": s.tail}))
        .collect();
    let segment_heads: Vec<String> = g.segment_heads.iter().map(|h| hex(h)).collect();

    json!({
        "blobs": blobs_json(&g),
        "diagnostics": g.diagnostics.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        "mode": "default",
        "quads": nquads.len(),
        "nquads": nquads,
        "opaque_reasons": g.opaque.iter().map(|o| o.reason.clone()).collect::<Vec<_>>(),
        "profiles": profiles,
        "segment_heads": segment_heads,
        "segments": g.segment_heads.len(),
        "streamable": streamable,
        "suppressions": g.suppressions.len(),
        "terms": g.terms.len(),
    })
}

/// Render `value` with `sort_keys=true`, one-space-indent JSON — matching
/// every other frozen `vectors/*.expected.json` (produced upstream by
/// gmeow-gts's `python/src/gts/vectors.py::expected_for`, `json.dump(...,
/// indent=1, sort_keys=True)`). `serde_json::Value::Object` is BTreeMap-backed
/// in this workspace (no `preserve_order` feature anywhere in the dependency
/// tree), so keys are already sorted; only the indent width needs matching.
fn render_expected_json(value: &Json) -> String {
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    serde::Serialize::serialize(value, &mut ser).expect("serialize expected.json");
    let mut text = String::from_utf8(buf).expect("serde_json emits UTF-8");
    text.push('\n');
    text
}

fn main() {
    let source = read_source();
    let vectors_dir = vectors_dir();

    let (pack, _cert) = compact_and_certify(
        &source,
        DictStrategy::None,
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
