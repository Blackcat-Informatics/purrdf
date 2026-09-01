// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A PINNED in-band dictionary through the CERTIFYING authoring path.
//!
//! `crates/gts/tests/pinned_dict_compaction.rs` holds the wire-level contract
//! (the caller's exact bytes in the header `"dct"` map, the `Dictionary_ID`
//! every primed frame declares, the blob-less and mixed-plan rulings). This
//! file adds the strongest round-trip instrument the workspace has: the
//! RDFC-1.0 content-refold digest, computed before and after the rewrite by
//! `compact_and_certify`, and independently re-checked by `verify_compaction`
//! together with the carried authorship signatures and the mandatory packaging
//! signature.
//!
//! "Folds to the same graph" is asserted at canonical-graph identity, not at
//! blob-bytes identity — a pinned dictionary must be a pure compression detail.

use std::collections::HashMap;

use purrdf_gts::compact::{DictPlan, DictStrategy};
use purrdf_gts::dict::raw_content_dict;
use purrdf_gts::model::{Term, TermKind};
use purrdf_gts::reader::{read, segment_append_state};
use purrdf_gts::writer::Writer;
use purrdf_rdf::gts_certify::{compact_and_certify, refold_digest, verify_compaction};
use purrdf_rdf::gts_dict_vectors::{TIMESTAMP, VECTOR_ZSTD_LEVEL, authorship_key, fixed_source};

/// The name the caller pins its shipped dictionary under.
const PINNED_NAME: &str = "shipped-bundle-v1";

fn packaging_key() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[7u8; 32])
}

fn keyring() -> HashMap<String, ed25519_dalek::VerifyingKey> {
    HashMap::from([
        ("authorA".to_string(), authorship_key().verifying_key()),
        ("pack".to_string(), packaging_key().verifying_key()),
    ])
}

/// The caller's SHIPPED dictionary, derived from a vocabulary unrelated to any
/// source compacted here so a re-derivation could never reproduce it.
fn shipped_dictionary() -> Vec<u8> {
    let corpus: Vec<Vec<u8>> = (0..400u32)
        .map(|i| {
            format!(
                "<https://example.org/slice/logic#c{}> <https://example.org/p/grounds> \
                 \"a shipped-vocabulary sentence unrelated to any packed content, {}\" .\n",
                i % 23,
                i
            )
            .into_bytes()
        })
        .collect();
    let refs: Vec<&[u8]> = corpus.iter().map(Vec::as_slice).collect();
    raw_content_dict(&refs, 8192).expect("the shipped dictionary builds")
}

/// A signed, BLOB-LESS source log: terms and quads only — the agent-memory
/// shape, which has no dictionary corpus at all.
fn blobless_source() -> Vec<u8> {
    let mut w = Writer::new("purrdf.gts");
    w.sign_with(authorship_key(), "authorA");
    let iri = |value: &str| Term {
        kind: TermKind::Iri,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    };
    let mut terms: Vec<Term> = (0..24u32)
        .map(|i| iri(&format!("https://example.org/memory/claim{i}")))
        .collect();
    terms.push(iri("https://example.org/memory/recalls"));
    terms.push(iri("https://example.org/memory/session"));
    w.add_terms(&terms);
    let quads: Vec<(usize, usize, usize, Option<usize>)> =
        (0..24usize).map(|s| (s, 24, 25, None)).collect();
    w.add_quads(&quads);
    w.add_index();
    w.into_bytes()
}

/// The header `"dct"` bytes a pack actually pinned, by name.
fn header_dicts(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
    segment_append_state(bytes)
        .expect("the pack header parses")
        .dicts
}

fn pinned_plan(dict: Vec<u8>) -> DictPlan {
    DictPlan::rsyncable(PINNED_NAME, DictStrategy::Pinned(dict), VECTOR_ZSTD_LEVEL)
}

fn certify(source: &[u8], plan: DictPlan) -> Vec<u8> {
    let (pack, cert) = compact_and_certify(
        source,
        plan,
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("a pinned-dictionary compaction certifies");
    assert_eq!(
        cert.pre_refold_digest, cert.post_refold_digest,
        "the certifying wrapper's own pre/post content-refold digests must agree"
    );
    pack
}

#[test]
fn a_pinned_pack_round_trips_at_canonical_graph_identity_and_verifies() {
    let shipped = shipped_dictionary();
    let source = fixed_source();
    let pack = certify(&source, pinned_plan(shipped.clone()));

    assert_eq!(
        header_dicts(&pack)[PINNED_NAME],
        shipped,
        "the certifying path pins the caller's exact bytes too"
    );

    let before = read(&source, true, None);
    let after = read(&pack, true, None);
    assert!(
        after.diagnostics.is_empty(),
        "the pinned pack must fold cleanly: {:?}",
        after.diagnostics
    );
    assert_eq!(
        refold_digest(&before).expect("source content-refold digest"),
        refold_digest(&after).expect("pack content-refold digest"),
        "the pack must fold to the SAME RDFC-1.0 canonical graph as its source"
    );

    let report = verify_compaction(&source, &pack, &keyring()).expect("verify_compaction runs");
    assert!(
        report.all_ok(),
        "a pinned-dictionary pack must independently verify — content equivalence, carried \
         authorship signatures, and the mandatory packaging signature: {report:?}"
    );
}

#[test]
fn a_blobless_pinned_pack_round_trips_and_verifies() {
    let shipped = shipped_dictionary();
    let source = blobless_source();
    assert!(
        read(&source, true, None).blobs.is_empty(),
        "the fixture must genuinely have NO content blobs"
    );

    let pack = certify(&source, pinned_plan(shipped.clone()));
    assert_eq!(
        header_dicts(&pack)[PINNED_NAME],
        shipped,
        "a blob-less pack pins the caller's exact bytes with no corpus in sight"
    );

    let report = verify_compaction(&source, &pack, &keyring()).expect("verify_compaction runs");
    assert!(
        report.all_ok(),
        "a blob-less pinned pack must independently verify: {report:?}"
    );
}

/// The frozen dict vectors are authored through this same wrapper: a pinned plan
/// must not perturb what a DERIVED plan produces over the same source.
#[test]
fn pinning_a_dictionary_does_not_perturb_a_derived_compaction_of_the_same_source() {
    let source = fixed_source();
    let derived_plan =
        || DictPlan::rsyncable(PINNED_NAME, DictStrategy::RawContent, VECTOR_ZSTD_LEVEL);
    let a = certify(&source, derived_plan());
    let _pinned = certify(&source, pinned_plan(shipped_dictionary()));
    let b = certify(&source, derived_plan());
    assert_eq!(
        a, b,
        "a derived compaction is byte-identical regardless of any pinned compaction \
         performed between the two"
    );
    assert_ne!(
        header_dicts(&a)[PINNED_NAME],
        shipped_dictionary(),
        "anti-tautology: the derived dictionary is not the shipped one"
    );
}
