// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The IR-boundary absoluteness invariant, exercised through **non-codec** ingress
//! only.
//!
//! Every ingress driven here reaches the term table without passing any text codec,
//! so nothing a codec-layer check could do would affect the outcome. That is the
//! point: the invariant has to live at the store-once interner, where *every* ingress
//! necessarily arrives, rather than being re-implemented once per codec seam where
//! each new seam is a fresh chance to forget it.
//!
//! Each case asserts the workspace's SHARED diagnostic code
//! (`purrdf_iri::IriError::diagnostic_code`), not a spelling invented for the IR.

use std::ops::ControlFlow;

use purrdf_core::ir::{
    BlankScope, DatasetSink, MutableDataset, QuadValues, RdfDatasetBuilder, TermValue, skolemize,
};
use purrdf_core::{DatasetMut, RdfLiteral, RdfQuad, RdfTerm};
use purrdf_events::{EventQuad, EventTerm, EventTermId, RdfEventSink};

/// The code `purrdf-iri` owns for "a relative IRI reference with no base in scope".
const RELATIVE: &str = "iri-relative-no-base";

/// Intern an absolute IRI, for the surrounding structure the cases need.
fn abs(b: &mut RdfDatasetBuilder, name: &str) -> purrdf_core::ir::TermId {
    b.intern_iri(&format!("http://example.org/{name}"))
}

/// The plainest non-codec ingress there is: intern a relative reference directly on
/// the builder and try to freeze.
#[test]
fn builder_refuses_to_freeze_a_relative_iri() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("foo");
    let p = abs(&mut b, "p");
    let o = abs(&mut b, "o");
    b.push_quad(s, p, o, None);

    let err = b.freeze().expect_err("a relative IRI cannot enter the IR");
    assert_eq!(err.code, RELATIVE);
    assert!(err.message.contains("\"foo\""), "{}", err.message);
    // The remedy is carried as its own field, not only baked into the message.
    assert!(
        err.detail.as_deref().is_some_and(|d| d.contains("@base")),
        "{err:?}"
    );
}

/// The empty IRI `<>` — the same-document reference — is a MISSING BASE, not an
/// "empty string" error. This is the exact shape that originally leaked through and
/// serialized as invalid N-Triples.
#[test]
fn builder_refuses_the_empty_iri() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("");
    let p = abs(&mut b, "p");
    let o = abs(&mut b, "o");
    b.push_quad(s, p, o, None);

    let err = b
        .freeze()
        .expect_err("the empty IRI is a relative reference");
    assert_eq!(err.code, RELATIVE);
}

/// The violation is recorded at INTERN time, so it is fatal even when the offending
/// term is never referenced by a quad. A freeze-time scan of only the *reachable*
/// terms would miss this.
#[test]
fn an_unreferenced_relative_iri_is_still_fatal() {
    let mut b = RdfDatasetBuilder::new();
    let s = abs(&mut b, "s");
    let p = abs(&mut b, "p");
    let o = abs(&mut b, "o");
    b.push_quad(s, p, o, None);
    let _orphan = b.intern_iri("../up");

    assert_eq!(
        b.freeze().expect_err("interned, therefore checked").code,
        RELATIVE
    );
}

/// Every IRI-bearing position is the same term table, so each one is covered by the
/// one check: predicate, graph name, a literal's datatype, and a quoted triple's
/// components.
#[test]
fn every_iri_position_is_covered() {
    // Predicate.
    let mut b = RdfDatasetBuilder::new();
    let s = abs(&mut b, "s");
    let p = b.intern_iri("relativePredicate");
    let o = abs(&mut b, "o");
    b.push_quad(s, p, o, None);
    assert_eq!(b.freeze().expect_err("predicate").code, RELATIVE);

    // Graph name.
    let mut b = RdfDatasetBuilder::new();
    let s = abs(&mut b, "s");
    let p = abs(&mut b, "p");
    let o = abs(&mut b, "o");
    let g = b.intern_iri("/graphs/1");
    b.push_quad(s, p, o, Some(g));
    assert_eq!(b.freeze().expect_err("graph name").code, RELATIVE);

    // Literal datatype — interned as an IRI term like any other.
    let mut b = RdfDatasetBuilder::new();
    let s = abs(&mut b, "s");
    let p = abs(&mut b, "p");
    let o = b.intern_literal(RdfLiteral::typed("42", "myDatatype"));
    b.push_quad(s, p, o, None);
    assert_eq!(b.freeze().expect_err("datatype").code, RELATIVE);

    // A quoted triple's own predicate (RDF 1.2 statement layer).
    let mut b = RdfDatasetBuilder::new();
    let s = abs(&mut b, "s");
    let p = abs(&mut b, "p");
    let o = abs(&mut b, "o");
    let inner_p = b.intern_iri("quotedPredicate");
    let triple = b.intern_triple(s, inner_p, o);
    b.push_quad(s, p, triple, None);
    assert_eq!(b.freeze().expect_err("quoted predicate").code, RELATIVE);
}

/// The owned-model boundary (`RdfTerm`/`RdfQuad`) is the path the language bindings'
/// quad constructors and the SHACL/rules re-materialization passes take. It reaches
/// the same interner, so it is covered without its own check.
#[test]
fn the_owned_model_boundary_is_covered() {
    let mut b = RdfDatasetBuilder::new();
    b.push_owned_quad(&RdfQuad {
        subject: RdfTerm::Iri("subjects/1".to_owned()),
        predicate: "http://example.org/p".to_owned(),
        object: RdfTerm::Iri("http://example.org/o".to_owned()),
        graph_name: None,
        location: None,
    });
    assert_eq!(b.freeze().expect_err("owned ingress").code, RELATIVE);
}

/// The permissive event-ingestion protocol — the shape GTS import and every
/// `RdfEventSink` producer drives — folds into the same builder, so a relative IRI
/// declared as an event term cannot be frozen either.
#[test]
fn the_event_ingest_path_is_covered() {
    let mut sink = DatasetSink::new();
    let (s, p, o) = (EventTermId(1), EventTermId(2), EventTermId(3));
    for (id, iri) in [
        (s, "notAbsolute"),
        (p, "http://example.org/p"),
        (o, "http://example.org/o"),
    ] {
        assert_eq!(
            sink.term(id, EventTerm::Iri(iri)).expect("declared"),
            ControlFlow::Continue(())
        );
    }
    assert_eq!(
        sink.quad(EventQuad { s, p, o, g: None }).expect("buffered"),
        ControlFlow::Continue(())
    );

    let err = sink
        .finish()
        .expect_err("the sink must not freeze a relative IRI");
    assert!(err.to_string().contains(RELATIVE), "{err}");
    assert!(sink.into_dataset().is_none(), "no dataset was produced");
}

/// A base with one absolute quad, for the overlay cases.
fn absolute_base() -> std::sync::Arc<purrdf_core::ir::RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = abs(&mut b, "s");
    let p = abs(&mut b, "p");
    let o = abs(&mut b, "o");
    b.push_quad(s, p, o, None);
    b.freeze().expect("the base dataset is absolute")
}

/// The copy-on-write mutable overlay refuses a relative IRI **at the insert that
/// names it**, not at some later freeze.
///
/// Freezing would refuse it too — that is the invariant, and the case below pins it —
/// but a mutable dataset may be mutated thousands of times before it is frozen, and
/// by then the error can no longer point at the call that introduced the bad term.
#[test]
fn the_mutable_overlay_refuses_at_the_insert_not_at_the_freeze() {
    let mut overlay = MutableDataset::new(absolute_base());
    let err = overlay
        .insert(QuadValues::triple(
            TermValue::Iri("http://example.org/s".to_owned()),
            TermValue::Iri("http://example.org/p".to_owned()),
            TermValue::Iri("relativeObject".to_owned()),
        ))
        .expect_err("the insert itself must refuse");
    assert_eq!(err.diagnostic_code(), RELATIVE);
    assert!(format!("{err}").contains("\"relativeObject\""), "{err}");

    // The refused quad did not land: the overlay is unchanged and still freezes.
    assert_eq!(overlay.added_len(), 0);
    let frozen = overlay.freeze().expect("a refused insert left no residue");
    assert_eq!(frozen.quad_count(), 1);
}

/// Every IRI-bearing position of an inserted quad is checked, including a literal's
/// datatype and an IRI nested inside a quoted triple (which the delta interns whole).
#[test]
fn the_overlay_checks_every_iri_position_of_an_inserted_quad() {
    let absolute = || TermValue::Iri("http://example.org/ok".to_owned());
    let cases: Vec<(&str, QuadValues)> = vec![
        (
            "subject",
            QuadValues::triple(
                TermValue::Iri("relativeSubject".to_owned()),
                absolute(),
                absolute(),
            ),
        ),
        (
            "predicate",
            QuadValues::triple(
                absolute(),
                TermValue::Iri("relativePredicate".to_owned()),
                absolute(),
            ),
        ),
        (
            "graph name",
            QuadValues::quad(
                absolute(),
                absolute(),
                absolute(),
                TermValue::Iri("relativeGraph".to_owned()),
            ),
        ),
        (
            "literal datatype",
            QuadValues::triple(
                absolute(),
                absolute(),
                TermValue::typed_literal("42", "relativeDatatype"),
            ),
        ),
        (
            "IRI nested in a quoted triple",
            QuadValues::triple(
                absolute(),
                absolute(),
                TermValue::Triple {
                    s: Box::new(absolute()),
                    p: Box::new(TermValue::Iri("relativeQuotedPredicate".to_owned())),
                    o: Box::new(absolute()),
                },
            ),
        ),
    ];
    for (position, quad) in cases {
        let mut overlay = MutableDataset::new(absolute_base());
        let Err(err) = overlay.insert(quad) else {
            panic!("a relative IRI in {position} position must be refused at insert");
        };
        assert_eq!(err.diagnostic_code(), RELATIVE, "{position}");
        assert_eq!(overlay.added_len(), 0, "{position} left residue");
    }
}

/// A blank-node label and a literal's lexical form are arbitrary strings: the
/// absoluteness rule applies to IRIs only and must not leak onto them.
#[test]
fn the_overlay_leaves_non_iri_strings_alone() {
    let mut overlay = MutableDataset::new(absolute_base());
    assert!(
        overlay
            .insert(QuadValues::triple(
                TermValue::blank("notAbsolute"),
                TermValue::Iri("http://example.org/p".to_owned()),
                TermValue::simple_literal("../also/not/an/iri"),
            ))
            .expect("blank labels and lexical forms are not IRIs")
    );
}

/// Absolute IRIs of every shape still freeze — the gate must not become a
/// scheme allowlist, and it must not reject the IRI forms the workspace already
/// mints (`urn:`, `blake3:`, `file:`, `.well-known/genid` skolems, non-ASCII).
#[test]
fn absolute_iris_of_every_shape_still_freeze() {
    let mut b = RdfDatasetBuilder::new();
    let p = abs(&mut b, "p");
    let s = abs(&mut b, "s");
    for iri in [
        "http://example.org/o",
        "https://example.org/a/b?q=1#f",
        "urn:uuid:0b7f0a1e-0000-4000-8000-000000000000",
        "file:///tmp/x",
        "http://example.org/caf\u{e9}",
        "http://example.org/.well-known/genid/b0",
        "did:example:123",
        "tag:example.org,2026:x",
    ] {
        let o = b.intern_iri(iri);
        b.push_quad(s, p, o, None);
    }
    let ds = b.freeze().expect("every absolute IRI is admissible");
    assert_eq!(ds.quad_count(), 8);
}

/// The gate is a PREDICATE, never a rewrite.
///
/// `check_absolute` parses only to decide "does this have a scheme"; it drops the
/// parsed `Iri` and the interner stores the original `&str` byte for byte. Nothing
/// here may apply RFC 3986 §5.2.4 dot-segment removal or §6.2.2 case/percent
/// normalization — that is syntax-based normalization, which RDF Concepts §3.2
/// forbids: identical document bytes must denote one graph with one canonical digest.
#[test]
fn the_gate_never_normalizes_the_iri_it_admits() {
    // Every one of these is absolute and every one has a shorter normalized form.
    let hostile = [
        "http://a/bb/ccc/../d;p?q",
        "http://example.org/a/./b/../c",
        "HTTP://EXAMPLE.ORG/Path",
        "http://example.org/%7Ename",
        "http://example.org/a//b",
        "http://example.org/.",
    ];
    let mut b = RdfDatasetBuilder::new();
    let p = abs(&mut b, "p");
    let s = abs(&mut b, "s");
    for iri in hostile {
        let o = b.intern_iri(iri);
        b.push_quad(s, p, o, None);
    }
    let ds = b.freeze().expect("all absolute, all admissible");

    // Read every IRI back out of the frozen term table and compare byte for byte.
    let mut seen: Vec<&str> = Vec::new();
    for raw in 0..ds.term_count() {
        if let purrdf_core::ir::TermRef::Iri(iri) =
            ds.resolve(purrdf_core::ir::TermId::from_index(raw as u32))
        {
            seen.push(iri);
        }
    }
    for iri in hostile {
        assert!(
            seen.contains(&iri),
            "{iri:?} was not interned verbatim; frozen IRIs were {seen:?}"
        );
    }
}

/// Skolemization mints IRIs internally, under a caller-supplied authority. Those
/// must satisfy the invariant like any other IRI — no exemption is needed or taken.
#[test]
fn skolem_minted_iris_satisfy_the_invariant() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_blank("b0", BlankScope::DEFAULT);
    let p = abs(&mut b, "p");
    let o = b.intern_blank("b1", BlankScope::DEFAULT);
    b.push_quad(s, p, o, None);
    let ds = b.freeze().expect("blank-only dataset freezes");

    let skolemized = skolemize(&ds, "http://example.org").expect("skolemize");
    // Reaching a frozen dataset at all IS the assertion: `skolemize` builds through
    // the same builder, so every minted `.well-known/genid` IRI passed the gate.
    assert_eq!(skolemized.quad_count(), 1);
}

/// A malformed IRI keeps its own precise code rather than being reported as a
/// missing base — telling a caller to add a `@base` would be a lie there.
#[test]
fn a_malformed_iri_is_not_reported_as_a_missing_base() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/a b");
    let p = abs(&mut b, "p");
    let o = abs(&mut b, "o");
    b.push_quad(s, p, o, None);

    let err = b.freeze().expect_err("a space is not an IRI character");
    assert_eq!(err.code, "iri-disallowed-char");
}

/// The gate must be reached on the miss path only. This cannot be observed as a
/// timing assertion (benches here are report-only), but it IS observable as
/// behavior: the FIRST violation is the one reported, no matter how many times the
/// offending string is subsequently re-interned, because re-interning is a hash hit
/// that never re-validates.
#[test]
fn only_the_first_violation_is_recorded_however_often_it_is_re_interned() {
    let mut b = RdfDatasetBuilder::new();
    let first = b.intern_iri("firstRelative");
    for _ in 0..1_000 {
        assert_eq!(b.intern_iri("firstRelative"), first, "hit path is stable");
    }
    let _second = b.intern_iri("secondRelative");
    let p = abs(&mut b, "p");
    let o = abs(&mut b, "o");
    b.push_quad(first, p, o, None);

    let err = b.freeze().expect_err("still fatal");
    assert_eq!(err.code, RELATIVE);
    assert!(err.message.contains("firstRelative"), "{}", err.message);
    assert!(!err.message.contains("secondRelative"), "{}", err.message);
}
