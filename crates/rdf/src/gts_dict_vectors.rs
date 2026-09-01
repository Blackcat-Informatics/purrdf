// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The fixed sources and authoring recipes behind the frozen in-band-dictionary
//! corpus vectors (`vectors/30-dict-rawcontent.gts`,
//! `vectors/31-dict-trained.gts`, `vectors/32-dict-rsyncable.gts`,
//! `vectors/33-multi-dict.gts`).
//!
//! ONE definition, shared by the maintainer freezing binary
//! (`src/bin/gen_dict_vectors.rs`) and the drift-guard test
//! (`tests/dict_vectors.rs`), so a regeneration and a drift check can never
//! start from different bytes. (The generator and the guard used to duplicate
//! the builder because a `[[bin]]` target exposes no importable surface — this
//! module is that surface.)
//!
//! It also hosts the shared cross-engine expected-fold oracle
//! ([`expected_fold_json`], [`expected_fold_json_in_mode`],
//! [`render_expected_json`]). The oracle is not dictionary-specific: it
//! reproduces the `<id>.expected.json` of EVERY vector in the frozen corpus,
//! which is what `tests/gts_corpus_expected_fold.rs` grades the whole corpus
//! against.

use std::collections::BTreeMap;

use ed25519_dalek::SigningKey;
use purrdf_gts::compact::{DEFAULT_DICT_NAME, DictPlan, DictStrategy};
use purrdf_gts::dict::raw_content_dict;
use purrdf_gts::model::Graph;
use purrdf_gts::reader::read;
use purrdf_gts::wire::{hex, map_get};
use purrdf_gts::writer::{FrameOptions, Writer, WriterOptions};
use serde_json::{Value as Json, json};

use crate::gts::dataset_from_gts_graph;
use crate::{SerializeGraph, serialize_dataset};

/// The fixed rewrite timestamp every dict vector is compacted under.
pub const TIMESTAMP: &str = "2026-01-01T00:00:00Z";
/// The zstd level the rsyncable/multi-dict vectors declare (§8.5 `level?`) —
/// the level GMEOW's frame profile mandates.
pub const VECTOR_ZSTD_LEVEL: i32 = 12;
/// The two dictionary names vector 33 pins.
pub const MULTI_DICT_NAMES: [&str; 2] = ["cats", "dogs"];

/// The fixed authorship signing key (`kid` "authorA") every dict-vector source
/// is signed with.
#[must_use]
pub fn authorship_key() -> SigningKey {
    SigningKey::from_bytes(&[3u8; 32])
}

/// The fixed packaging signing key (`kid` "pack") every dict-vector pack is
/// packaged with.
#[must_use]
pub fn packaging_key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

/// A fixed, signed GTS source: 40 content-blob frames of repeated structure
/// (a `<s{i%7}> <p> "dict vector claim {i} about cats"` N-Triples line per
/// blob), signed under the fixed authorship key, closed with an `index`
/// footer — a stable corpus a pack dictionary strategy has real structure to
/// train on.
#[must_use]
pub fn fixed_source() -> Vec<u8> {
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

/// A larger, more redundant source than [`fixed_source`]: 300 content blobs
/// of long, near-identical text.
///
/// [`fixed_source`]'s 40-blob corpus exists to freeze small, fast-folding
/// vectors, not to demonstrate a net size win — its blob content is small
/// enough that the pinned dictionary's own bytes can outweigh what 40 tiny
/// frames individually save. This corpus has enough repeated structure for the
/// dictionary's one-time cost to be genuinely amortized.
#[must_use]
pub fn size_comparison_source() -> Vec<u8> {
    let mut w = Writer::new("purrdf.gts");
    w.sign_with(authorship_key(), "authorA");
    for i in 0..300u32 {
        let blob = format!(
            "<https://example.org/s{}> <https://example.org/p> \"dict vector claim {} about \
             cats and the shared structure repeated across every blob in this redundant \
             corpus, which a pack dictionary should compress extremely well\" .\n",
            i % 7,
            i
        )
        .into_bytes();
        w.add_blob_owned(blob, Some("text/plain"), None);
    }
    w.add_index();
    w.into_bytes()
}

/// The plan behind `vectors/32-dict-rsyncable.gts`: ONE raw-content dictionary
/// priming a `zstd-rsyncable` chain at level 12 — GMEOW's mandated frame
/// profile (exactly one transform per payload frame, `zstd-rsyncable`, level
/// 12), with the dictionary priming every independent block.
///
/// The raw-content producer is used rather than FastCOVER so the vector is
/// byte-frozen cross-platform (no transcendental floating point).
#[must_use]
pub fn rsyncable_plan() -> DictPlan {
    DictPlan::rsyncable(
        DEFAULT_DICT_NAME,
        DictStrategy::RawContent,
        VECTOR_ZSTD_LEVEL,
    )
}

/// One corpus for each of vector 33's two dictionaries: deliberately disjoint
/// vocabularies, so the two pinned dictionaries have genuinely different bytes
/// and per-frame selection is observable rather than decorative.
#[must_use]
pub fn multi_dict_corpora() -> [Vec<Vec<u8>>; 2] {
    let build = |topic: &str, predicate: &str| -> Vec<Vec<u8>> {
        (0..200u32)
            .map(|i| {
                format!(
                    "<https://example.org/{topic}/s{}> <https://example.org/{predicate}> \
                     \"multi-dict vector claim {} about {topic}\" .\n",
                    i % 11,
                    i
                )
                .into_bytes()
            })
            .collect()
    };
    [build("cats", "purrs"), build("dogs", "barks")]
}

/// Author `vectors/33-multi-dict.gts`: ONE pack pinning TWO named in-band
/// dictionaries, with per-frame selection between them.
///
/// Frames alternate between the two dictionaries, so the pack exercises what
/// no single-dictionary pack can: a catalog carrying several `zstd-rsyncable`
/// entries that differ only by their `dct`, a `"x"` chain per frame naming the
/// right one, and a reader resolving each frame against the dictionary it was
/// actually primed with.
///
/// # Panics
/// Panics when the fixed corpora fail to produce a dictionary or the writer
/// rejects the fixed configuration — both are authoring bugs in this file, not
/// runtime conditions.
#[must_use]
pub fn multi_dict_pack() -> Vec<u8> {
    let corpora = multi_dict_corpora();
    let dicts: Vec<(String, Vec<u8>)> = MULTI_DICT_NAMES
        .iter()
        .zip(corpora.iter())
        .map(|(name, corpus)| {
            let refs: Vec<&[u8]> = corpus.iter().map(Vec::as_slice).collect();
            (
                (*name).to_string(),
                raw_content_dict(&refs, 4096).expect("raw-content dictionary builds"),
            )
        })
        .collect();

    let mut w = Writer::with_options(
        "purrdf.gts",
        WriterOptions {
            dicts,
            zstd_level: Some(VECTOR_ZSTD_LEVEL),
            ..WriterOptions::default()
        },
    )
    .expect("multi-dict writer configures");
    w.sign_with(authorship_key(), "authorA");

    for (index, corpus) in corpora.iter().enumerate() {
        let name = MULTI_DICT_NAMES[index];
        for (row, blob) in corpus.iter().enumerate().take(8) {
            let mut payload = blob.clone();
            // Pad past a single rsyncable block on the last row of each group so
            // the vector also freezes a MULTI-block dict-primed frame.
            if row == 7 {
                while payload.len() <= 70_000 {
                    payload.extend_from_slice(blob);
                }
            }
            w.add_blob_transformed(
                payload,
                Some("text/plain"),
                Some(name),
                &["zstd-rsyncable".to_string()],
                Some(name),
            )
            .expect("dict-primed rsyncable blob frame writes");
        }
    }
    // A baseline frame that names NO dictionary, so the pack proves the plain
    // catalog entry and the dict-bound entries coexist in one file.
    w.add_frame_with_options(
        "meta",
        FrameOptions {
            payload: Some(ciborium::value::Value::Map(vec![(
                "vector".into(),
                "33-multi-dict".into(),
            )])),
            transform: vec!["zstd-rsyncable".to_string()],
            ..FrameOptions::default()
        },
    )
    .expect("baseline meta frame writes");
    w.add_index();
    w.into_bytes()
}

/// Build the standard cross-engine expected-fold JSON for a GTS vector.
///
/// The schema and rendering match
/// `gmeow-gts::python/src/gts/vectors.py::expected_for`: decoded blob metadata,
/// sorted N-Quads, diagnostics, profiles, segment heads, streaming coverage,
/// suppressions, and term/quad counts. The value is derived from the supplied
/// bytes' own verified fold, never from the authoring inputs.
///
/// # Panics
///
/// Panics when `pack` does not fold cleanly, a blob cannot be decoded, blob
/// media-type metadata is absent, or the folded graph cannot be projected to
/// native N-Quads. Those conditions make a positive conformance vector invalid.
#[must_use]
pub fn expected_fold_json(pack: &[u8]) -> Json {
    let graph = read(pack, true, None);
    assert!(
        graph.diagnostics.is_empty(),
        "the conformance vector must fold cleanly: {:?}",
        graph.diagnostics
    );
    fold_json(&graph, DEFAULT_MODE)
}

/// The vector-corpus read mode that folds a file as a whole, segments included.
pub const DEFAULT_MODE: &str = "default";
/// The vector-corpus read mode that refuses segmentation, so a segmented file
/// stops at the first boundary and reports `SegmentBoundary`.
pub const PRE_SEGMENT_MODE: &str = "pre-segment";

/// Build the expected-fold JSON for a vector read under the corpus `mode` it
/// declares, tolerating diagnostics.
///
/// [`expected_fold_json`] covers the clean-folding positive vectors the
/// dictionary corpus is made of. The frozen corpus also contains negative
/// vectors, which fold WITH diagnostics, and `pre-segment` vectors, which must
/// be read with segmentation refused; both are oracles too, and this is the
/// entry point that can reproduce them.
///
/// # Panics
///
/// Panics under the same invalid-vector conditions as [`expected_fold_json`]:
/// an undecodable blob, absent blob media-type metadata, or a folded graph that
/// will not project to native N-Quads.
#[must_use]
pub fn expected_fold_json_in_mode(pack: &[u8], mode: &str) -> Json {
    let graph = read(pack, mode != PRE_SEGMENT_MODE, None);
    fold_json(&graph, mode)
}

/// The shared body behind both entry points.
fn fold_json(graph: &Graph, mode: &str) -> Json {
    let nquads = nquads_sorted(graph);
    // The corpus schema counts BASE quads here, not projected N-Quads lines:
    // the statement layer (reifier and annotation rows) also serializes to
    // N-Quads, so the two numbers part company on any vector that carries one.
    let quad_count = graph.quads.len();
    let mut profiles = graph.segment_profiles.clone();
    profiles.sort();
    profiles.dedup();
    let streamable: Vec<Json> = graph
        .segment_streamable
        .iter()
        .map(|state| {
            json!({
                "claimed": state.claimed,
                "covered": state.covered,
                "tail": state.tail,
            })
        })
        .collect();
    let segment_heads: Vec<String> = graph.segment_heads.iter().map(|head| hex(head)).collect();

    json!({
        "blobs": blobs_json(graph),
        "diagnostics": graph
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.clone())
            .collect::<Vec<_>>(),
        "mode": mode,
        "nquads": nquads,
        "opaque_reasons": graph
            .opaque
            .iter()
            .map(|opaque| opaque.reason.clone())
            .collect::<Vec<_>>(),
        "profiles": profiles,
        "quads": quad_count,
        "segment_heads": segment_heads,
        "segments": graph.segment_heads.len(),
        "streamable": streamable,
        "suppressions": graph.suppressions.len(),
        "terms": graph.terms.len(),
    })
}

/// Render an expected-fold value with sorted keys, one-space indentation, and
/// one trailing newline, matching the shared GTS vector corpus byte format.
///
/// # Panics
///
/// Panics only if `serde_json` cannot serialize its own in-memory value or
/// emits non-UTF-8 bytes, both of which are invariant violations.
#[must_use]
pub fn render_expected_json(value: &Json) -> String {
    let mut bytes = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut bytes, formatter);
    serde::Serialize::serialize(value, &mut serializer).expect("serialize expected-fold JSON");
    let mut text = String::from_utf8(bytes).expect("serde_json emits UTF-8");
    text.push('\n');
    text
}

/// Folded blob metadata as `digest -> {mt, size}`.
fn blobs_json(graph: &Graph) -> Json {
    let mut blobs = BTreeMap::new();
    for (digest, entry) in &graph.blobs {
        let media_type = graph
            .blob_meta
            .iter()
            .find(|(candidate, _)| candidate == digest)
            .and_then(|(_, metadata)| match metadata {
                ciborium::value::Value::Map(entries) => match map_get(entries, "mt") {
                    Some(ciborium::value::Value::Text(value)) => Some(value.clone()),
                    _ => None,
                },
                _ => None,
            })
            .unwrap_or_else(|| panic!("blob {digest} has no declared \"mt\" metadata"));
        let size = entry
            .decoded_len()
            .unwrap_or_else(|error| panic!("blob {digest} decodes: {error}"));
        blobs.insert(digest.clone(), json!({"mt": media_type, "size": size}));
    }
    Json::Object(blobs.into_iter().collect())
}

/// Project the folded graph through the production bridge and serializer.
fn nquads_sorted(graph: &Graph) -> Vec<String> {
    let dataset = dataset_from_gts_graph(graph)
        .unwrap_or_else(|error| panic!("GTS-to-dataset bridge: {error}"));
    let bytes = serialize_dataset(&dataset, "application/n-quads", SerializeGraph::Dataset)
        .unwrap_or_else(|error| panic!("serialize N-Quads: {error:?}"));
    let text = String::from_utf8(bytes).expect("N-Quads serializer emits UTF-8");
    let mut lines: Vec<String> = text
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    lines.sort();
    lines
}
