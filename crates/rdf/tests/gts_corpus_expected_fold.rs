// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Whole-corpus drift guard: every frozen GTS vector against its own
//! `<id>.expected.json` fold oracle.
//!
//! `vectors/` is the shared cross-engine GTS corpus, byte-frozen and governed
//! upstream in `gmeow-gts`; this repository never regenerates, edits, or
//! deletes it. Each `<id>.gts` ships an `<id>.expected.json` stating the fold
//! every conforming engine must produce. Reading a vector but not its oracle
//! leaves an engine's agreement with the corpus asserted only in prose, and a
//! disagreement no check observes is a swallowed error — so this guard folds
//! every vector through the production reader and N-Quads projection and
//! compares the rendered oracle byte-for-byte with the committed file.
//!
//! The vector's own `mode` selects how it is read: `pre-segment` vectors refuse
//! segmentation and stop at the first boundary; everything else folds whole.
//!
//! [`KNOWN_DIVERGENCES`] is the one escape hatch, and it is held to XPASS
//! discipline: a listed vector must STILL disagree. An entry that starts
//! agreeing fails this test, so a divergence can never be closed upstream and
//! left stale here. Each entry carries a dedicated test elsewhere that pins
//! both sides of the disagreement; this list only records that the corpus-wide
//! equality is knowingly relaxed for it.

use std::path::PathBuf;

use purrdf_rdf::gts_dict_vectors::{
    DEFAULT_MODE, expected_fold_json_in_mode, render_expected_json,
};
use serde_json::Value as Json;

/// Every vector in the frozen corpus, so an upstream add or remove is loud.
const VECTOR_COUNT: usize = 39;

/// Vectors whose committed expectation this engine knowingly contradicts.
///
/// `12-conflicting-reifier` binds one reifier id to two triples and carries no
/// quoted-triple term. `rdf:reifies` is not a functional property, so both
/// bindings are legitimate and this reader keeps both without complaint; the
/// committed expectation still states the superseded single-binding reading —
/// one N-Quad plus a `ConflictingReifier` diagnostic — which encodes the wrong
/// premise. Correcting the expectation is an upstream act on the governed
/// corpus. `purrdf-gts`'s `frozen_conflicting_reifier_divergence` test pins
/// both sides in full and documents what must land upstream.
const KNOWN_DIVERGENCES: [&str; 1] = ["12-conflicting-reifier"];

fn vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vectors")
}

#[test]
fn every_frozen_vector_matches_its_committed_expected_fold() {
    let vectors = vectors_dir();
    let mut stems: Vec<String> = std::fs::read_dir(&vectors)
        .expect("vectors directory")
        .map(|entry| entry.expect("vector entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "gts"))
        .map(|path| {
            path.file_stem()
                .expect("vector file stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    stems.sort();
    assert_eq!(
        stems.len(),
        VECTOR_COUNT,
        "the frozen GTS corpus changed size; every vector must be graded",
    );

    let mut still_diverging: Vec<&str> = Vec::new();
    for stem in &stems {
        let pack = std::fs::read(vectors.join(format!("{stem}.gts")))
            .unwrap_or_else(|error| panic!("read {stem}.gts: {error}"));
        let expected = std::fs::read_to_string(vectors.join(format!("{stem}.expected.json")))
            .unwrap_or_else(|error| panic!("read {stem}.expected.json: {error}"));
        let declared: Json = serde_json::from_str(&expected)
            .unwrap_or_else(|error| panic!("parse {stem}.expected.json: {error}"));
        let mode = declared["mode"].as_str().unwrap_or(DEFAULT_MODE);

        let produced = render_expected_json(&expected_fold_json_in_mode(&pack, mode));
        let known = KNOWN_DIVERGENCES.contains(&stem.as_str());
        if known {
            assert_ne!(
                produced, expected,
                "{stem} is listed as a known divergence but now AGREES — the \
                 divergence closed upstream; drop it from KNOWN_DIVERGENCES and \
                 turn its dedicated test into a plain agreement check",
            );
            still_diverging.push(
                KNOWN_DIVERGENCES
                    .iter()
                    .find(|name| *name == stem)
                    .expect("matched entry"),
            );
        } else {
            assert_eq!(
                produced, expected,
                "{stem}.expected.json must be the fold oracle this engine \
                 produces for the frozen bytes",
            );
        }
    }

    assert_eq!(
        still_diverging.len(),
        KNOWN_DIVERGENCES.len(),
        "every known divergence must name a vector that is actually in the corpus",
    );
}
