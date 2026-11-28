// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! left stale here. The sole entry must agree on every field except N-Quads
//! spelling, and even there the two datasets must be isomorphic.
//!
//! The fold prints a `GTS-VECTORS:` scoreboard line, which is what
//! `scripts/conformance-matrix.py` scrapes for the corpus's row in the umbrella
//! `make conformance` matrix. The line is emitted unconditionally, before any
//! assertion, so a red run still reports what it measured.

use std::collections::BTreeSet;

use purrdf_core::{CANON_CORPUS_POLICY, CanonicalizationControl};
use purrdf_lex::json::Value;
use purrdf_rdf::gts_dict_vectors::{
    DEFAULT_MODE, expected_fold_json_in_mode, render_expected_json,
};
use purrdf_rdf::{datasets_isomorphic, parse_dataset};

#[path = "support/vectors.rs"]
mod vectors;
use vectors::vectors_dir;

/// Every vector in the frozen corpus, so an upstream add or remove is loud.
const VECTOR_COUNT: usize = 39;

/// Vectors whose committed expectation this engine knowingly contradicts.
///
/// The oracle spells anonymous entries as ordinary authored labels. The
/// scoped projection keeps those entries distinct from all authored labels.
const KNOWN_DIVERGENCES: [&str; 1] = ["15b-anon-bnode-union"];

fn assert_blank_spelling_only(mut produced: Value, declared: &Value, frozen: &str) {
    fn dataset(quads: &Value) -> std::sync::Arc<purrdf_rdf::ir::RdfDataset> {
        let lines = quads.as_array().expect("oracle N-Quads array");
        let mut document = String::new();
        for line in lines {
            document.push_str(line.as_str().expect("one N-Quads line"));
            document.push('\n');
        }
        parse_dataset(document.as_bytes(), "application/n-quads", None)
            .expect("valid oracle N-Quads")
    }

    let actual = dataset(&produced["nquads"]);
    let expected = dataset(&declared["nquads"]);
    assert_eq!(
        produced["nquads"]
            .as_array()
            .expect("produced N-Quads")
            .len(),
        declared["nquads"].as_array().expect("frozen N-Quads").len(),
        "the projection retains the number of lines",
    );
    assert!(
        datasets_isomorphic(
            &actual,
            &expected,
            &mut CanonicalizationControl::new(CANON_CORPUS_POLICY, None),
        )
        .expect("compare frozen and projected anonymous-node datasets"),
        "the anonymous blank-node divergence is only a change of spelling",
    );
    *produced.get_mut("nquads").expect("produced N-Quads field") = declared["nquads"].clone();
    assert_eq!(
        render_expected_json(&produced),
        frozen,
        "all other frozen oracle bytes agree",
    );
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

    let mut diverging: BTreeSet<String> = BTreeSet::new();
    let mut spelling_only = None;
    let mut mismatch: Option<(String, String, String)> = None;
    for stem in &stems {
        let pack = std::fs::read(vectors.join(format!("{stem}.gts")))
            .unwrap_or_else(|error| panic!("read {stem}.gts: {error}"));
        let expected = std::fs::read_to_string(vectors.join(format!("{stem}.expected.json")))
            .unwrap_or_else(|error| panic!("read {stem}.expected.json: {error}"));
        let declared = purrdf_lex::json::read(&expected)
            .unwrap_or_else(|error| panic!("parse {stem}.expected.json: {error}"));
        let mode = declared["mode"].as_str().unwrap_or(DEFAULT_MODE);

        let document = expected_fold_json_in_mode(&pack, mode);
        let produced = render_expected_json(&document);
        if produced != expected {
            diverging.insert(stem.clone());
            // Keep the FIRST unexpected pair whole, so the failure below can
            // show the two documents rather than only their names.
            if KNOWN_DIVERGENCES.contains(&stem.as_str()) {
                spelling_only = Some((document, declared, expected));
            } else if mismatch.is_none() {
                mismatch = Some((stem.clone(), produced, expected));
            }
        }
    }

    // The umbrella matrix's row for this corpus. Printed BEFORE the assertions so
    // it is emitted whether the corpus agrees or not.
    println!(
        "GTS-VECTORS: agreed {} total {} diverging {}",
        stems.len() - diverging.len(),
        stems.len(),
        diverging.len(),
    );

    if let Some((produced, declared, frozen)) = spelling_only {
        assert_blank_spelling_only(produced, &declared, &frozen);
    }

    if let Some((stem, produced, expected)) = mismatch {
        assert_eq!(
            produced, expected,
            "{stem}.expected.json must be the fold oracle this engine produces \
             for the frozen bytes",
        );
    }

    let known: BTreeSet<&str> = KNOWN_DIVERGENCES.into_iter().collect();
    let diverging: BTreeSet<&str> = diverging.iter().map(String::as_str).collect();
    assert_eq!(
        diverging, known,
        "the divergence ledger must name exactly the vectors that actually \
         disagree: an entry that started AGREEING closed upstream (drop it from \
         KNOWN_DIVERGENCES), and an unlisted disagreement is a regression in this reader",
    );
}
