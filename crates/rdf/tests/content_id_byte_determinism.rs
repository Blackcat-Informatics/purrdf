// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Byte-determinism gate for content-addressed terms.
//!
//! Content addressing adds three DERIVED, non-serialized structures to a frozen
//! [`RdfDataset`]: the `content_ids` side table (interned-IRI → digest), the
//! resolved derivation-predicate `TermId`, and the lazy predecessor index. None
//! of them is part of the RDF term/quad/reifier/annotation content, so enabling
//! recognition MUST NOT perturb any serializer's output.
//!
//! This test builds the SAME logical dataset two ways — plain
//! ([`RdfDatasetBuilder::new`]) and content-addressing active
//! ([`RdfDatasetBuilder::with_content_addressing`]) — serializes both through the
//! canonical RDFC-1.0 N-Quads surface ([`canonical_flat_nquads`]), and asserts the
//! bytes are identical. To prove determinism DESPITE recognition being live (not
//! because it silently no-op'd), it also asserts the content-addressing dataset
//! actually recognized its `blake3:<64hex>` terms.

use std::sync::Arc;

use purrdf_core::ContentIdScheme;
use purrdf_rdf::gts_compose::SnapshotBuilder;
use purrdf_rdf::{
    BlankScope, CanonError, CanonHash, CompositeDatasetView, CompositeSource, RESERVED_NAMESPACE,
    RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTextDirection, TermId, TermPosition,
    ViewCanonError, ViewLimits, blank_count_view, canonical_flat_nquads, canonicalize_with,
    check_admissible_flat_view, flat_dataset_from_quads, flat_rdf_quads_from_dataset,
    parse_dataset, try_canonicalize_flat_graph_view, try_canonicalize_flat_view,
    try_canonicalize_view,
};

/// The caller-supplied derivation-predicate IRI (no fabricated vocabulary: this is
/// configuration, spelled under `example.org` per the test-fixture rule).
const DERIVED_FROM: &str = "http://example.org/wasDerivedFrom";

/// A `blake3:`-scheme content-id IRI whose 64-hex tail is the two-hex-digit
/// `pair` repeated 32× (e.g. `"aa"` → `blake3:aaaa…aa`, 64 hex chars).
fn blake3_iri(pair: &str) -> String {
    format!("blake3:{}", pair.repeat(32))
}

/// The `TermId`s of the two content-addressed IRIs, captured at build time so the
/// caller can probe the frozen side table.
struct CaIds {
    subject: TermId,
    object: TermId,
}

/// Intern one fixed logical dataset into `builder` and return the two
/// content-addressed term ids. Identical push sequence for both builders, so any
/// serialized difference can only come from the content-addressing config itself.
///
/// Content exercised:
/// - a `blake3:<64hex>` IRI in SUBJECT position,
/// - a different `blake3:<64hex>` IRI in OBJECT position,
/// - an ordinary `example.org` quad, and
/// - a derivation annotation `(ca_subject, wasDerivedFrom, ca_object)`.
fn populate(builder: &mut RdfDatasetBuilder) -> CaIds {
    let ca_subject = builder.intern_iri(&blake3_iri("aa"));
    let ca_object = builder.intern_iri(&blake3_iri("bb"));
    let plain_subject = builder.intern_iri("http://example.org/thing");
    let predicate = builder.intern_iri("http://example.org/p");
    let derived_from = builder.intern_iri(DERIVED_FROM);
    let label = builder.intern_literal(RdfLiteral::simple("content-addressed"));

    // A content-addressed IRI as the subject of an asserted quad.
    builder.push_quad(ca_subject, predicate, label, None);
    // A content-addressed IRI as the object of an asserted quad.
    builder.push_quad(plain_subject, predicate, ca_object, None);
    // A derivation annotation linking the two content-addressed terms.
    builder.push_annotation(ca_subject, derived_from, ca_object);

    CaIds {
        subject: ca_subject,
        object: ca_object,
    }
}

/// Enabling content addressing must not change one canonical byte of serialized
/// output, because the recognition state is a derived side table — never content.
#[test]
fn content_addressing_does_not_perturb_serialized_bytes() {
    // (A) Plain builder — content addressing inactive.
    let mut plain = RdfDatasetBuilder::new();
    let _ = populate(&mut plain);
    let dataset_a = plain.freeze().expect("plain dataset freezes");

    // (B) Content-addressing active, with the derivation predicate configured.
    let scheme = ContentIdScheme::new("blake3:").expect("':' is not a hex digit");
    let mut addressed =
        RdfDatasetBuilder::with_content_addressing(scheme, Some(DERIVED_FROM.to_string()));
    let ca = populate(&mut addressed);
    let dataset_b = addressed
        .freeze()
        .expect("content-addressed dataset freezes");

    // Sanity: recognition was ACTIVE — the blake3 terms landed in the side table.
    // (Without this, the byte-equality below could pass trivially if recognition
    // had silently done nothing.)
    assert!(
        dataset_b.content_id(ca.subject).is_some(),
        "the blake3 subject IRI must be recognized as a content-id term"
    );
    assert!(
        dataset_b.content_id(ca.object).is_some(),
        "the blake3 object IRI must be recognized as a content-id term"
    );
    // The plain dataset, by contrast, has an empty side table.
    assert_eq!(
        dataset_a.content_ids().count(),
        0,
        "the plain dataset must not recognize any content-id term"
    );
    assert_eq!(
        dataset_b.content_ids().count(),
        2,
        "exactly the two blake3 IRIs are content-addressed"
    );

    // The load-bearing assertion: identical logical content → identical bytes,
    // regardless of the (non-serialized) content-addressing side tables.
    let bytes_a = canonical_flat_nquads(&dataset_a).expect("canonicalize plain");
    let bytes_b = canonical_flat_nquads(&dataset_b).expect("canonicalize addressed");
    assert_eq!(
        bytes_a.as_bytes(),
        bytes_b.as_bytes(),
        "content-addressing changed the serialized bytes:\n--- plain ---\n{bytes_a}\n--- addressed ---\n{bytes_b}"
    );
}

/// A single-source composite view over `dataset`, in an explicitly SHARED blank
/// identity space so the view reports the source's own `(label, scope)` pairs.
fn composite_over(dataset: &Arc<RdfDataset>) -> CompositeDatasetView {
    CompositeDatasetView::from_shared_sources(
        vec![CompositeSource::new(Arc::clone(dataset))],
        ViewLimits::default(),
    )
    .expect("a single retained source composes")
}

/// LITERAL NORMALIZATION IS A BYTE TRAP, and the two GTS ingestion surfaces must
/// fall into it identically.
///
/// A view's `TermRef::Literal` ALWAYS carries a datatype term id, while the flat
/// snapshot tables store a language-tagged literal with NO datatype and fold
/// `xsd:string` away entirely. So the view path has to DROP three datatype IRIs —
/// `xsd:string`, `rdf:langString`, `rdf:dirLangString` — and, because it drops
/// them, must never intern them either. Interning even one adds a dictionary row,
/// which re-ids every term after it and moves `snapshot_content_id`.
#[test]
fn the_view_ingestion_path_drops_the_same_datatype_iris_as_the_flat_path() {
    let source = parse_dataset(
        concat!(
            "<http://example.org/s> <http://example.org/a> \"bare\" .\n",
            "<http://example.org/s> <http://example.org/b> ",
            "\"x\"^^<http://www.w3.org/2001/XMLSchema#string> .\n",
            "<http://example.org/s> <http://example.org/c> \"cat\"@en .\n",
            "<http://example.org/s> <http://example.org/d> \"مرحبا\"@ar--rtl .\n",
            // A datatype that is NOT implicit must still be interned, or the test
            // would pass by dropping everything.
            "<http://example.org/s> <http://example.org/e> ",
            "\"7\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
        )
        .as_bytes(),
        "application/n-triples",
        None,
    )
    .expect("the literal spread parses");

    let mut flat = SnapshotBuilder::new();
    flat.add_dataset(&source).expect("flat ingests");
    let mut view = SnapshotBuilder::new();
    let _ = view
        .add_view(&composite_over(&source))
        .expect("the view path ingests");

    assert_eq!(
        flat.snapshot_content_id(),
        view.snapshot_content_id(),
        "the two ingestion surfaces disagreed on the literal traps"
    );
    assert_eq!(flat.snapshot_payload(), view.snapshot_payload());

    let rendered = format!("{:?}", view.snapshot_payload());
    for dropped in [
        "XMLSchema#string",
        "22-rdf-syntax-ns#langString",
        "22-rdf-syntax-ns#dirLangString",
    ] {
        assert!(
            !rendered.contains(dropped),
            "the implicit datatype {dropped:?} must not reach the term table: {rendered}"
        );
    }
    // NON-VACUITY: an explicit, non-implicit datatype IS interned, so the
    // assertions above are about normalization rather than about a term table
    // that simply holds no datatypes.
    assert!(
        rendered.contains("XMLSchema#integer"),
        "an explicit datatype must still be interned: {rendered}"
    );
    assert!(
        rendered.contains("rtl"),
        "the directional literal keeps its base direction: {rendered}"
    );
}

// ============================================================================
// The flat-assertion presentation's flat/native wrapper-agreement differential,
// identity algebra, and sentinel-absence corpus.
//
// The oracle below reproduces the PRE-DELEGATION body of
// `canonical_flat_nquads_with`, exactly as it stood at commit `863cf2f8`
// (before `eb7863e2` made it delegate to `purrdf_core::try_canonicalize_flat_view`;
// see `git show 863cf2f8:crates/rdf/src/native_quads.rs`):
//
//     let flat = flat_dataset_from_quads(&flat_rdf_quads_from_dataset(dataset))?;
//     Ok(crate::canonicalize_with(&flat, hash).nquads)
//
// composed here from the SAME surviving helpers this crate still exports
// (`flat_rdf_quads_from_dataset`, `flat_dataset_from_quads`,
// `purrdf_core::canonicalize_with` — re-exported as `canonicalize_with`),
// entirely independent of `try_canonicalize_flat_view` itself. A comparison
// against the now-delegating `canonical_flat_nquads_with` would be a
// self-comparison and prove nothing; this route shares no code with the
// function under test below the two un-fold/re-freeze helpers.
// ============================================================================

/// The flat/native wrapper-agreement differential oracle: the pre-delegation
/// flatten-and-canonicalize route.
fn reference_flat_canon(dataset: &RdfDataset, hash: CanonHash) -> String {
    let flat = flat_dataset_from_quads(&flat_rdf_quads_from_dataset(dataset))
        .expect("an already-valid dataset's own flat quad stream must re-freeze");
    canonicalize_with(&flat, hash).nquads
}

/// Assert `try_canonicalize_flat_view` agrees with [`reference_flat_canon`] on
/// `dataset`, under BOTH RDFC-1.0 hash algorithms.
fn assert_flat_differential(dataset: &RdfDataset, case: &str) {
    for hash in [CanonHash::Sha256, CanonHash::Sha384] {
        let actual = try_canonicalize_flat_view(dataset, hash)
            .unwrap_or_else(|e| panic!("{case} ({hash:?}) must be admissible: {e}"))
            .nquads;
        let expected = reference_flat_canon(dataset, hash);
        assert_eq!(
            actual, expected,
            "{case} ({hash:?}): try_canonicalize_flat_view diverged from the \
             pre-delegation flatten-and-canonicalize route"
        );
    }
}

/// An `example.org` IRI (no fabricated vocabulary; test fixtures use
/// `example.org` per the workspace rule).
fn ex(local: &str) -> String {
    format!("http://example.org/{local}")
}

// ── §1: fixed differential corpus, one named case each ──────────────────────

/// Quads in the default graph AND in a named graph.
#[test]
fn differential_default_and_named_graph_worlds() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&ex("s"));
    let p = b.intern_iri(&ex("p"));
    let o = b.intern_iri(&ex("o"));
    b.push_quad(s, p, o, None);
    let g = b.intern_iri(&ex("g"));
    let s2 = b.intern_iri(&ex("s2"));
    b.push_quad(s2, p, o, Some(g));
    let ds = b.freeze().expect("valid dataset");
    assert_flat_differential(&ds, "default+named graph worlds");
}

/// The SAME blank node co-referenced from the default graph and a named graph.
#[test]
fn differential_blank_coreference_across_graphs() {
    let mut b = RdfDatasetBuilder::new();
    let shared = b.intern_blank("shared", BlankScope::DEFAULT);
    let p = b.intern_iri(&ex("p"));
    let o = b.intern_iri(&ex("o"));
    b.push_quad(shared, p, o, None);
    let g = b.intern_iri(&ex("g"));
    b.push_quad(shared, p, o, Some(g));
    let ds = b.freeze().expect("valid dataset");
    assert_flat_differential(&ds, "blank co-reference across graphs");
}

/// Two DIFFERENT blank nodes that carry the SAME local label `"n"` in two
/// different scopes, produced by [`RdfDataset::union`]'s standardize-apart
/// merge (each input claims its own fresh [`BlankScope`]) — the collision a
/// label-keyed (rather than scope-keyed) canonicalizer would silently conflate.
#[test]
fn differential_blank_label_collision_across_scopes() {
    /// A fresh dataset with one blank labelled `"n"` in its own scope — building
    /// two of these with different `local_p` predicates and unioning them is how
    /// this test produces two DIFFERENT blanks sharing one label across scopes.
    fn one_labeled(local_p: &str) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let n = b.intern_blank("n", BlankScope::DEFAULT);
        let p = b.intern_iri(&ex(local_p));
        let o = b.intern_iri(&ex("o"));
        b.push_quad(n, p, o, None);
        b.freeze().expect("valid dataset")
    }
    let a = one_labeled("p1");
    let b_ds = one_labeled("p2");
    let unioned = RdfDataset::union(&[&a, &b_ds]);
    assert_flat_differential(&unioned, "blank label collision across scopes");
}

/// Language-tagged AND base-direction literals.
#[test]
fn differential_language_and_direction_literals() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&ex("s"));
    let p1 = b.intern_iri(&ex("p1"));
    let tagged = b.intern_literal(RdfLiteral::language_tagged("hello", "en"));
    b.push_quad(s, p1, tagged, None);
    let p2 = b.intern_iri(&ex("p2"));
    let directional = b.intern_literal(RdfLiteral {
        direction: Some(RdfTextDirection::Rtl),
        ..RdfLiteral::language_tagged("مرحبا", "ar")
    });
    b.push_quad(s, p2, directional, None);
    let ds = b.freeze().expect("valid dataset");
    assert_flat_differential(&ds, "language + direction literals");
}

/// A triple term nested three levels deep, asserted as a top-level quad's object.
#[test]
fn differential_deeply_quoted_triple_terms() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&ex("s"));
    let p = b.intern_iri(&ex("p"));
    let o = b.intern_iri(&ex("o"));
    let t1 = b.intern_triple(s, p, o);
    let s2 = b.intern_iri(&ex("s2"));
    let p2 = b.intern_iri(&ex("p2"));
    let t2 = b.intern_triple(s2, p2, t1);
    let s3 = b.intern_iri(&ex("s3"));
    let p3 = b.intern_iri(&ex("p3"));
    let t3 = b.intern_triple(s3, p3, t2);
    let top_s = b.intern_iri(&ex("top"));
    let top_p = b.intern_iri(&ex("holds"));
    b.push_quad(top_s, top_p, t3, None);
    let ds = b.freeze().expect("valid dataset");
    assert_flat_differential(&ds, "deeply quoted triple terms");
}

/// A bare reifier declaration.
#[test]
fn differential_reifiers() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&ex("s"));
    let p = b.intern_iri(&ex("p"));
    let o = b.intern_iri(&ex("o"));
    let t = b.intern_triple(s, p, o);
    let r = b.intern_blank("r", BlankScope::DEFAULT);
    b.push_reifier(r, t);
    let ds = b.freeze().expect("valid dataset");
    assert_flat_differential(&ds, "reifiers");
}

/// Annotations over a reifier.
#[test]
fn differential_annotations() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&ex("s"));
    let p = b.intern_iri(&ex("p"));
    let o = b.intern_iri(&ex("o"));
    let t = b.intern_triple(s, p, o);
    let r = b.intern_blank("r", BlankScope::DEFAULT);
    b.push_reifier(r, t);
    let conf = b.intern_iri(&ex("confidence"));
    let high = b.intern_iri(&ex("high"));
    b.push_annotation(r, conf, high);
    let ds = b.freeze().expect("valid dataset");
    assert_flat_differential(&ds, "annotations");
}

/// The reifier+annotation case, pinned EXPLICITLY under both SHA-256 and
/// SHA-384 (not merely through the shared both-hashes helper), per the
/// flat/native wrapper-agreement differential's own requirement that this
/// exact case run under both algorithms.
#[test]
fn differential_reifier_and_annotation_case_under_both_hash_algorithms() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&ex("s"));
    let p = b.intern_iri(&ex("p"));
    let o = b.intern_iri(&ex("o"));
    let t = b.intern_triple(s, p, o);
    let r = b.intern_blank("r", BlankScope::DEFAULT);
    b.push_reifier(r, t);
    let conf = b.intern_iri(&ex("confidence"));
    let high = b.intern_iri(&ex("high"));
    b.push_annotation(r, conf, high);
    let ds = b.freeze().expect("valid dataset");

    let sha256_actual = try_canonicalize_flat_view(&*ds, CanonHash::Sha256)
        .expect("admissible")
        .nquads;
    assert_eq!(
        sha256_actual,
        reference_flat_canon(&ds, CanonHash::Sha256),
        "reifier+annotation under SHA-256"
    );

    let sha384_actual = try_canonicalize_flat_view(&*ds, CanonHash::Sha384)
        .expect("admissible")
        .nquads;
    assert_eq!(
        sha384_actual,
        reference_flat_canon(&ds, CanonHash::Sha384),
        "reifier+annotation under SHA-384"
    );
    // On this trivial single-blank graph the rendered N-Quads TEXT happens to be
    // hash-algorithm-independent (the algorithm only feeds the n-degree tie-break
    // search, never exercised here) — `sha256_actual == sha384_actual` is
    // therefore EXPECTED, not a test bug; both were checked against the oracle
    // under their OWN algorithm above, which is the load-bearing assertion.
}

/// `cdt:List` AND `cdt:Map` composite literals, each with an embedded
/// `BLANK_NODE_LABEL` token — a blank node denoted from INSIDE a literal's
/// lexical form rather than a term position.
#[test]
fn differential_composite_literals_with_embedded_blank_labels() {
    let text = concat!(
        "<http://example.org/s> <http://example.org/p1> ",
        "\"[_:b, 1, _:b]\"^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List> .\n",
        "<http://example.org/s> <http://example.org/p2> ",
        "\"{ '1': _:c, '2': 2 }\"^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map> .\n",
    );
    let ds = parse_dataset(text.as_bytes(), "application/n-triples", None)
        .expect("the composite literal fixtures parse");
    assert_flat_differential(
        &ds,
        "cdt:List/cdt:Map composite literals with embedded blank labels",
    );
}

/// A row spelled BOTH as a reifier-declaration side-table entry AND, separately,
/// as its own ordinary base quad — the flat presentation's id-level dedup law —
/// exercised in the default graph and, independently, in a named graph.
#[test]
fn differential_row_spelled_natively_and_as_base_quad_in_both_graph_placements() {
    const REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

    // Default graph.
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&ex("s"));
    let p = b.intern_iri(&ex("p"));
    let o = b.intern_iri(&ex("o"));
    let t = b.intern_triple(s, p, o);
    let r = b.intern_blank("r", BlankScope::DEFAULT);
    b.push_reifier(r, t);
    let reifies = b.intern_iri(REIFIES);
    b.push_quad(r, reifies, t, None);
    let default_ds = b.freeze().expect("valid dataset");
    assert_flat_differential(
        &default_ds,
        "row spelled natively and as base quad: default graph",
    );

    // Named graph.
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&ex("s"));
    let p = b.intern_iri(&ex("p"));
    let o = b.intern_iri(&ex("o"));
    let t = b.intern_triple(s, p, o);
    let r = b.intern_blank("r", BlankScope::DEFAULT);
    let g = b.intern_iri(&ex("g"));
    b.push_reifier_in_graph(r, t, Some(g));
    let reifies = b.intern_iri(REIFIES);
    b.push_quad(r, reifies, t, Some(g));
    let named_ds = b.freeze().expect("valid dataset");
    assert_flat_differential(
        &named_ds,
        "row spelled natively and as base quad: named graph",
    );
}

// ── §2: identity algebra (executable laws, both directions) ─────────────────

/// A dataset whose statement layer (a reifier binding + an annotation) is
/// populated independently in BOTH the default graph and a named graph — the
/// shared fixture for every identity-algebra law below that needs non-vacuous
/// statement-layer content in more than one scope.
fn statement_layer_fixture_both_graphs() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let conf = b.intern_iri(&ex("confidence"));
    let high = b.intern_iri(&ex("high"));

    let s1 = b.intern_iri(&ex("s1"));
    let p1 = b.intern_iri(&ex("p1"));
    let o1 = b.intern_iri(&ex("o1"));
    b.push_quad(s1, p1, o1, None);
    let t1 = b.intern_triple(s1, p1, o1);
    let r1 = b.intern_blank("r1", BlankScope::DEFAULT);
    b.push_reifier(r1, t1);
    b.push_annotation(r1, conf, high);

    let g = b.intern_iri(&ex("g"));
    let s2 = b.intern_iri(&ex("s2"));
    let p2 = b.intern_iri(&ex("p2"));
    let o2 = b.intern_iri(&ex("o2"));
    b.push_quad(s2, p2, o2, Some(g));
    let t2 = b.intern_triple(s2, p2, o2);
    let r2 = b.intern_blank("r2", BlankScope::DEFAULT);
    b.push_reifier_in_graph(r2, t2, Some(g));
    b.push_annotation_in_graph(r2, conf, high, Some(g));

    b.freeze().expect("valid dataset")
}

/// (a) `flat == overlay` IFF the dataset carries no reifier/annotation rows —
/// both directions, as separate assertions.
#[test]
fn flat_and_overlay_agree_without_a_statement_layer_and_diverge_with_one() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&ex("s"));
    let p = b.intern_iri(&ex("p"));
    let o = b.intern_iri(&ex("o"));
    b.push_quad(s, p, o, None);
    let plain = b.freeze().expect("valid dataset");
    let flat_bytes = try_canonicalize_flat_view(&*plain, CanonHash::Sha256)
        .expect("admissible")
        .nquads;
    let overlay_bytes = try_canonicalize_view(&*plain, CanonHash::Sha256)
        .expect("admissible")
        .nquads;
    assert_eq!(
        flat_bytes, overlay_bytes,
        "a statement-layer-free dataset must canonicalize identically under both \
         presentations"
    );

    let statement_bearing = statement_layer_fixture_both_graphs();
    let flat_bytes = try_canonicalize_flat_view(&*statement_bearing, CanonHash::Sha256)
        .expect("admissible")
        .nquads;
    let overlay_bytes = try_canonicalize_view(&*statement_bearing, CanonHash::Sha256)
        .expect("admissible")
        .nquads;
    assert_ne!(
        flat_bytes, overlay_bytes,
        "a statement-bearing dataset must diverge between the flat and overlay \
         presentations"
    );
}

/// (b) Flat idempotence under re-ingestion, through BOTH loaders this crate
/// actually offers for re-entering a flat canonical document:
///
/// - Loader A — the TEXT codec path: the canonical N-Quads STRING is parsed
///   back in through [`parse_dataset`] (`application/n-quads`), the crate's
///   one N-Quads text ingestion surface (the same one
///   `the_view_ingestion_path_drops_the_same_datatype_iris_as_the_flat_path`
///   above uses).
/// - Loader B — the flat-QUADS path: the SAME canonical document's native
///   [`purrdf_rdf::RdfQuad`] row stream ([`flat_rdf_quads_from_dataset`], the
///   un-fold that produces exactly the rows the text serializer prints) is
///   re-frozen WITHOUT folding via [`flat_dataset_from_quads`] — the
///   sanctioned way to re-enter an already-flat quad stream as a dataset.
///
/// Both loaders are reachable; a third ("flat quads parsed from raw text
/// without going through `parse_dataset`") does not exist as a separate
/// surface in this crate — every text ingestion path is `parse_dataset`.
#[test]
fn flat_canon_is_idempotent_under_both_reachable_reingestion_loaders() {
    let ds = statement_layer_fixture_both_graphs();
    for hash in [CanonHash::Sha256, CanonHash::Sha384] {
        let first = try_canonicalize_flat_view(&*ds, hash)
            .expect("fixture is admissible")
            .nquads;
        assert!(
            !first.is_empty(),
            "fixture must actually canonicalize to content"
        );

        // Loader A: text.
        let reparsed_text = parse_dataset(first.as_bytes(), "application/n-quads", None)
            .expect("the crate's own canonical flat output must re-parse as N-Quads");
        let second_via_text = try_canonicalize_flat_view(&*reparsed_text, hash)
            .expect("re-parsed dataset is admissible")
            .nquads;
        assert_eq!(
            second_via_text, first,
            "re-ingesting the flat canonical document through the TEXT codec path \
             must reproduce the same bytes ({hash:?})"
        );

        // Loader B: flat quads (no text involved).
        let flat_quads = flat_rdf_quads_from_dataset(&ds);
        let reingested_quads = flat_dataset_from_quads(&flat_quads)
            .expect("the dataset's own flat quad stream must re-freeze");
        let second_via_quads = try_canonicalize_flat_view(&*reingested_quads, hash)
            .expect("re-frozen dataset is admissible")
            .nquads;
        assert_eq!(
            second_via_quads, first,
            "re-ingesting through the flat-QUADS path must reproduce the same bytes \
             ({hash:?})"
        );
    }
}

/// (c) Scope commutation, extended to a fixture with statement-layer rows in
/// BOTH graphs: `try_canonicalize_flat_graph_view` on graph `g` equals
/// `try_canonicalize_flat_view` of `g`'s own projection, even though a
/// SEPARATE, independent statement layer also lives in the default graph.
#[test]
fn flat_graph_scope_commutes_with_statement_layer_rows_present_in_both_graphs() {
    let ds = statement_layer_fixture_both_graphs();
    let g = ex("g");

    let via_scope = try_canonicalize_flat_graph_view(&*ds, &g, CanonHash::Sha256)
        .expect("g is admissible")
        .nquads;
    let projected = ds.project_named_graph(&g);
    let via_projection = try_canonicalize_flat_view(&projected, CanonHash::Sha256)
        .expect("g's projection is admissible")
        .nquads;
    assert_eq!(
        via_scope, via_projection,
        "scope commutation must hold with an independent statement layer present in \
         a NEIGHBOUR graph too"
    );
    assert!(
        !via_scope.is_empty(),
        "the selected graph must actually carry content"
    );
    // Isolation: the default graph's independent statement layer must not leak in.
    assert!(
        !via_scope.contains(&ex("s1")),
        "the OTHER scope's statement-layer content must not leak into this graph's \
         flat canon: {via_scope}"
    );
}

/// (d) Blank-count presentation invariance: the blank population the
/// canonicalizer enumerates is identical under the overlay's
/// [`blank_count_view`] and the flat canonical document's own `_:c14nN` label
/// set.
#[test]
fn blank_count_is_invariant_between_flat_and_overlay_presentations() {
    let ds = statement_layer_fixture_both_graphs();
    let overlay_count = blank_count_view(&*ds);
    let flat_bytes = try_canonicalize_flat_view(&*ds, CanonHash::Sha256)
        .expect("admissible")
        .nquads;
    let flat_labels: std::collections::BTreeSet<&str> = flat_bytes
        .lines()
        .flat_map(str::split_whitespace)
        .filter(|token| token.starts_with("_:c14n"))
        .collect();
    assert!(
        !flat_labels.is_empty(),
        "the fixture must actually carry blank nodes, or this law proves nothing: \
         {flat_bytes}"
    );
    assert_eq!(
        flat_labels.len(),
        overlay_count,
        "the blank population the canonicalizer enumerates must be identical under \
         both presentations"
    );
}

// ── §5: sentinel-absence assertion (executable) ──────────────────────────────

/// `rdf:reifies` — the real predicate a reifier binding denotes once lowered to the
/// flat-assertion presentation. Named locally (not exported by `purrdf-rdf`, which
/// mints no vocabulary): every crate that needs this literal spells it itself.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// [`statement_layer_fixture_both_graphs`], plus a base quad spelling a THIRD
/// reifier — over a triple that appears nowhere else in the fixture — through the
/// overlay's own sentinel, exactly the shape `ex:r <urn:purrdf:rdfc:reifies> <<(
/// ex:s ex:p ex:o )>>` spells.
///
/// Without this quad, [`flat_canon_never_mints_the_reserved_namespace_and_the_row_count_is_exact`]'s
/// assertion that no reserved-namespace text appears in the output was true
/// vacuously: [`statement_layer_fixture_both_graphs`] carries no sentinel-spelled
/// base quad at all, so a leak in exactly that fold path had no way to reach the
/// assertion. This fixture gives it one.
fn statement_layer_fixture_both_graphs_with_a_spelled_row() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let conf = b.intern_iri(&ex("confidence"));
    let high = b.intern_iri(&ex("high"));

    let s1 = b.intern_iri(&ex("s1"));
    let p1 = b.intern_iri(&ex("p1"));
    let o1 = b.intern_iri(&ex("o1"));
    b.push_quad(s1, p1, o1, None);
    let t1 = b.intern_triple(s1, p1, o1);
    let r1 = b.intern_blank("r1", BlankScope::DEFAULT);
    b.push_reifier(r1, t1);
    b.push_annotation(r1, conf, high);

    let g = b.intern_iri(&ex("g"));
    let s2 = b.intern_iri(&ex("s2"));
    let p2 = b.intern_iri(&ex("p2"));
    let o2 = b.intern_iri(&ex("o2"));
    b.push_quad(s2, p2, o2, Some(g));
    let t2 = b.intern_triple(s2, p2, o2);
    let r2 = b.intern_blank("r2", BlankScope::DEFAULT);
    b.push_reifier_in_graph(r2, t2, Some(g));
    b.push_annotation_in_graph(r2, conf, high, Some(g));

    // The third row: NO native reifier of its own — it exists ONLY as a base quad
    // spelling the overlay's sentinel shape.
    let s3 = b.intern_iri(&ex("s3"));
    let p3 = b.intern_iri(&ex("p3"));
    let o3 = b.intern_iri(&ex("o3"));
    let t3 = b.intern_triple(s3, p3, o3);
    let r3 = b.intern_iri(&ex("r3"));
    let reifies_sentinel = b.intern_iri(&format!("{RESERVED_NAMESPACE}reifies"));
    b.push_quad(r3, reifies_sentinel, t3, None);

    b.freeze().expect("valid dataset")
}

/// On a dataset carrying reifiers AND annotations: (a) zero occurrences of the
/// reserved namespace string in the flat output, and (b) the line count equals
/// the exact expected flat row count (ordinary quads + one row per reifier +
/// one row per annotation) — run on BOTH the all-native fixture and a fixture that
/// also carries a sentinel-SPELLED base quad, so the law is falsifiable rather than
/// vacuously true (see
/// [`statement_layer_fixture_both_graphs_with_a_spelled_row`]'s documentation).
#[test]
fn flat_canon_never_mints_the_reserved_namespace_and_the_row_count_is_exact() {
    let ds = statement_layer_fixture_both_graphs();
    let bytes = try_canonicalize_flat_view(&*ds, CanonHash::Sha256)
        .expect("admissible")
        .nquads;

    assert!(
        !bytes.contains(RESERVED_NAMESPACE),
        "the flat presentation must never mint the overlay's reserved sentinel \
         namespace {RESERVED_NAMESPACE:?}: {bytes}"
    );

    // 2 base quads + 2 reifier rows (r1, r2) + 2 annotation rows = 6, by the
    // fixture's own construction above (no `(s, p, o, g)` collisions between them).
    let line_count = bytes.lines().filter(|line| !line.is_empty()).count();
    assert_eq!(
        line_count, 6,
        "expected exactly 6 flat rows (2 base quads + 2 reifier rows + 2 \
         annotation rows): {bytes}"
    );

    // The falsifiable variant: a sentinel-spelled base quad is present too.
    let spelled_ds = statement_layer_fixture_both_graphs_with_a_spelled_row();
    let spelled_bytes = try_canonicalize_flat_view(&*spelled_ds, CanonHash::Sha256)
        .expect("admissible")
        .nquads;

    assert!(
        !spelled_bytes.contains(RESERVED_NAMESPACE),
        "the flat presentation must never mint the overlay's reserved sentinel, \
         EVEN when the input itself carries a sentinel-spelled base quad — the \
         regression this guards is a sentinel-spelled row leaking the sentinel \
         under the flat presentation: {spelled_bytes}"
    );
    assert!(
        spelled_bytes.contains(RDF_REIFIES),
        "the sentinel-spelled row must have actually lowered to an ordinary \
         rdf:reifies row rather than being silently dropped: {spelled_bytes}"
    );
    let spelled_line_count = spelled_bytes
        .lines()
        .filter(|line| !line.is_empty())
        .count();
    assert_eq!(
        spelled_line_count, 7,
        "expected exactly 7 flat rows (the previous 6 plus the one sentinel-spelled \
         reifier row, lowered): {spelled_bytes}"
    );
}

/// (c) A valid neighbour: an IRI adjacent to, but OUTSIDE, the reserved
/// namespace is ADMITTED with that IRI present verbatim — and the reserved
/// namespace itself is refused TYPED as `Refused(ReservedVocabulary…)`.
///
/// Paired the OTHER way too, per the over-refusal rule: the SAME reserved predicate
/// over a plain IRI object stays refused, but over a TRIPLE-TERM object — the exact
/// shape the overlay's own lowering emits — it is now ADMITTED and lowers to
/// `rdf:reifies` rather than being refused. Both neighbours are checked so this test
/// cannot pass by having simply widened the refusal into an admission of everything.
#[test]
fn a_neighbouring_iri_is_admitted_while_the_reserved_namespace_is_typed_refused() {
    // Valid neighbour: `urn:purrdf:other:…`, NOT inside `urn:purrdf:rdfc:`.
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&ex("s"));
    let neighbour = "urn:purrdf:other:annotation";
    let p = b.intern_iri(neighbour);
    let o = b.intern_iri(&ex("o"));
    b.push_quad(s, p, o, None);
    let admitted_ds = b.freeze().expect("valid dataset");
    let bytes = try_canonicalize_flat_view(&*admitted_ds, CanonHash::Sha256)
        .expect("an IRI adjacent to, but outside, the reserved namespace must be admitted")
        .nquads;
    assert!(
        bytes.contains(neighbour),
        "the neighbouring IRI must be present verbatim in the admitted output: {bytes}"
    );
    assert!(check_admissible_flat_view(&*admitted_ds).is_ok());

    // The reserved namespace itself: refused, TYPED, naming the offending
    // position.
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&ex("s"));
    let reserved = format!("{RESERVED_NAMESPACE}reifies");
    let p = b.intern_iri(&reserved);
    let o = b.intern_iri(&ex("o"));
    b.push_quad(s, p, o, None);
    let inadmissible_ds = b.freeze().expect("valid dataset");
    match try_canonicalize_flat_view(&*inadmissible_ds, CanonHash::Sha256) {
        Err(ViewCanonError::Refused(CanonError::ReservedVocabulary(violation))) => {
            assert_eq!(violation.iri.as_ref(), reserved.as_str());
            assert_eq!(violation.position, TermPosition::Predicate);
        }
        other => panic!("expected a typed ReservedVocabulary refusal; got {other:?}"),
    }
    assert!(check_admissible_flat_view(&*inadmissible_ds).is_err());

    // The neighbour in the OTHER direction: the identical reserved predicate, but
    // over a TRIPLE-TERM object instead of a plain IRI — the exact shape the
    // overlay's own lowering emits — is now ADMITTED (not refused) and lowers to an
    // ordinary `rdf:reifies` row rather than leaking the sentinel.
    let mut b = RdfDatasetBuilder::new();
    let r = b.intern_iri(&ex("r"));
    let s = b.intern_iri(&ex("s"));
    let p = b.intern_iri(&ex("p"));
    let o = b.intern_iri(&ex("o"));
    let triple = b.intern_triple(s, p, o);
    let reifies = b.intern_iri(&reserved);
    b.push_quad(r, reifies, triple, None);
    let admitted_triple_term_ds = b.freeze().expect("valid dataset");
    let bytes = try_canonicalize_flat_view(&*admitted_triple_term_ds, CanonHash::Sha256)
        .expect(
            "the reserved predicate over a TRIPLE-TERM object is the fold's own \
             emitted shape and must be admitted, not refused",
        )
        .nquads;
    assert!(
        !bytes.contains(RESERVED_NAMESPACE),
        "the admitted, lowered row must not carry the sentinel: {bytes}"
    );
    assert!(
        bytes.contains("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"),
        "the admitted row must lower to the real rdf:reifies predicate: {bytes}"
    );
    assert!(check_admissible_flat_view(&*admitted_triple_term_ds).is_ok());
}
