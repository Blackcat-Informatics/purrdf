// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Regression test for the succinct-pack-codec dictionary's single unified
//! id-space fix (issue tracked as "the pack dict must mint one id per
//! distinct `TermValue`, regardless of role").
//!
//! The `DatasetView` seam (`crates/rdf-core/src/dataset_view.rs`) exposes ONE
//! position-agnostic value->id method, `term_id_by_value`, and the SPARQL
//! evaluator resolves every pattern constant — subject, predicate, AND
//! object — through it before calling `quads_for_pattern`. An earlier
//! revision of [`purrdf_core::ir::pack::dict::PackDict`] used a classic-HDT
//! split id space instead: `id_by_value` searched only non-predicate
//! sections and `predicate_id_by_value` searched only the predicate
//! section, so a "pure predicate" (a term used ONLY as a predicate) had NO
//! id via `id_by_value`, and a dual-role term (both a predicate AND a
//! subject/object somewhere) got TWO different ids. Either would break the
//! seam: a pure-predicate pattern constant would wrongly resolve to `None`,
//! and a dual-role term's id would not match its own predicate-position
//! occurrences.
//!
//! This test builds a dataset with exactly those two shapes — a dual-role
//! term `p` (predicate in some triples, subject/object in others) and a
//! pure-predicate term `q` (predicate only) — and proves the fixed
//! dictionary mints ONE id for each, resolvable via either lookup method,
//! and that querying [`TriplesRef::pattern`] with that single id — exactly
//! how the evaluator uses the seam — returns precisely the expected quads
//! in EVERY position the id occurs.

use std::collections::HashSet;

use purrdf_core::ir::pack::dict::PackDict;
use purrdf_core::ir::pack::triples::{Triples, TriplesRef};
use purrdf_core::{GraphMatch, RdfDataset, RdfDatasetBuilder, TermValue};

/// An `example.org` IRI value.
fn iri(name: &str) -> TermValue {
    TermValue::iri(format!("http://example.org/{name}"))
}

/// One resolved quad, in dataset-independent `TermValue` form.
type ValueQuad = (TermValue, TermValue, TermValue, Option<TermValue>);

/// Brute-force reference: every quad in `dataset`, resolved to
/// dataset-independent `TermValue`s, matching the value-level pattern (each
/// `None` axis unbound) — independent of any pack/dictionary machinery under
/// test.
fn brute_force_pattern(
    dataset: &RdfDataset,
    s: Option<&TermValue>,
    p: Option<&TermValue>,
    o: Option<&TermValue>,
) -> HashSet<ValueQuad> {
    dataset
        .quads()
        .map(|q| {
            (
                dataset.term_value(q.s),
                dataset.term_value(q.p),
                dataset.term_value(q.o),
                q.g.map(|g| dataset.term_value(g)),
            )
        })
        .filter(|(sv, pv, ov, _)| {
            s.is_none_or(|v| sv == v) && p.is_none_or(|v| pv == v) && o.is_none_or(|v| ov == v)
        })
        .collect()
}

/// The pack codec's answer for the same value-level pattern, resolved through
/// [`PackDict::id_by_value`] exactly as the SPARQL evaluator's
/// `DatasetView::term_id_by_value` seam does for EVERY pattern position
/// (subject, predicate, or object alike — see the module docs).
fn pack_pattern(
    dict: &PackDict,
    triples: &TriplesRef<'_>,
    s: Option<&TermValue>,
    p: Option<&TermValue>,
    o: Option<&TermValue>,
) -> HashSet<ValueQuad> {
    let s_id = s.map(|v| dict.id_by_value(v).expect("subject constant resolves"));
    let p_id = p.map(|v| dict.id_by_value(v).expect("predicate constant resolves"));
    let o_id = o.map(|v| dict.id_by_value(v).expect("object constant resolves"));
    triples
        .pattern(s_id, p_id, o_id, GraphMatch::Any)
        .map(|(s, p, o, g)| {
            (
                dict.term_value(s),
                dict.term_value(p),
                dict.term_value(o),
                g.map(|gid| dict.term_value(gid)),
            )
        })
        .collect()
}

/// Build the fixture: `p` plays BOTH a predicate role (in the first two
/// quads) AND a subject/object role (in the third and fourth); `q` plays
/// ONLY a predicate role (fifth and sixth quads) — never a subject or
/// object anywhere in the dataset.
fn build_fixture() -> std::sync::Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();

    let s1 = b.intern_iri("http://example.org/s1");
    let p = b.intern_iri("http://example.org/p");
    let o1 = b.intern_iri("http://example.org/o1");
    b.push_quad(s1, p, o1, None); // p as predicate

    let s2 = b.intern_iri("http://example.org/s2");
    let o2 = b.intern_iri("http://example.org/o2");
    b.push_quad(s2, p, o2, None); // p as predicate again

    let p2 = b.intern_iri("http://example.org/p2");
    let o3 = b.intern_iri("http://example.org/o3");
    b.push_quad(p, p2, o3, None); // p as SUBJECT

    let s3 = b.intern_iri("http://example.org/s3");
    let p3 = b.intern_iri("http://example.org/p3");
    b.push_quad(s3, p3, p, None); // p as OBJECT

    let s4 = b.intern_iri("http://example.org/s4");
    let q = b.intern_iri("http://example.org/q");
    let o4 = b.intern_iri("http://example.org/o4");
    b.push_quad(s4, q, o4, None); // q as predicate, pure

    let s5 = b.intern_iri("http://example.org/s5");
    let o5 = b.intern_iri("http://example.org/o5");
    b.push_quad(s5, q, o5, None); // q as predicate, pure (again)

    b.freeze().expect("valid dataset")
}

#[test]
fn dual_role_term_gets_one_id_agreeing_across_both_lookup_methods() {
    let dataset = build_fixture();
    let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("dict opens");

    let p_value = iri("p");
    let via_id_by_value = dict
        .id_by_value(&p_value)
        .expect("dual-role term resolves via id_by_value");
    let via_predicate_id_by_value = dict
        .predicate_id_by_value(&p_value)
        .expect("dual-role term resolves via predicate_id_by_value too");
    assert_eq!(
        via_id_by_value, via_predicate_id_by_value,
        "id_by_value and predicate_id_by_value must return the SAME id for a term \
         that is both a predicate and a subject/object"
    );
}

#[test]
fn pure_predicate_resolves_via_id_by_value_and_agrees_with_predicate_id_by_value() {
    let dataset = build_fixture();
    let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("dict opens");

    let q_value = iri("q");
    let via_id_by_value = dict
        .id_by_value(&q_value)
        .expect("a pure predicate MUST resolve via id_by_value — this is the exact bug fixed");
    let via_predicate_id_by_value = dict
        .predicate_id_by_value(&q_value)
        .expect("pure predicate resolves via predicate_id_by_value too");
    assert_eq!(
        via_id_by_value, via_predicate_id_by_value,
        "id_by_value and predicate_id_by_value must agree for a pure predicate"
    );
}

/// The full seam simulation: resolve `p`'s and `q`'s single unified ids via
/// [`PackDict::id_by_value`] — exactly as `DatasetView::term_id_by_value`
/// would for ANY pattern position — and confirm [`TriplesRef::pattern`]
/// returns precisely the quads a brute-force scan of the source dataset
/// finds, for `p` used as a SUBJECT, `p` used as a PREDICATE, and `q` used as
/// a PREDICATE.
#[test]
fn single_id_matches_both_subject_and_predicate_position_occurrences() {
    let dataset = build_fixture();
    let dict_bytes = PackDict::encode(&dataset).to_bytes();
    let dict = PackDict::open(&dict_bytes).expect("dict opens");
    let triples_bytes = Triples::encode(&dict, &dataset).to_bytes();
    let triples = TriplesRef::from_bytes(&triples_bytes).expect("triples open");

    let p_value = iri("p");
    let q_value = iri("q");

    // `(p, ?, ?)` — p used as a SUBJECT constant. Exactly one quad: (p, p2, o3).
    let expected_p_subject = brute_force_pattern(&dataset, Some(&p_value), None, None);
    let actual_p_subject = pack_pattern(&dict, &triples, Some(&p_value), None, None);
    assert_eq!(actual_p_subject, expected_p_subject);
    assert_eq!(
        actual_p_subject.len(),
        1,
        "exactly one quad has p as subject"
    );

    // `(?, p, ?)` — p used as a PREDICATE constant. Exactly two quads.
    let expected_p_predicate = brute_force_pattern(&dataset, None, Some(&p_value), None);
    let actual_p_predicate = pack_pattern(&dict, &triples, None, Some(&p_value), None);
    assert_eq!(actual_p_predicate, expected_p_predicate);
    assert_eq!(
        actual_p_predicate.len(),
        2,
        "exactly two quads have p as predicate"
    );

    // `(?, ?, p)` — p used as an OBJECT constant, for completeness. Exactly one quad.
    let expected_p_object = brute_force_pattern(&dataset, None, None, Some(&p_value));
    let actual_p_object = pack_pattern(&dict, &triples, None, None, Some(&p_value));
    assert_eq!(actual_p_object, expected_p_object);
    assert_eq!(actual_p_object.len(), 1, "exactly one quad has p as object");

    // Subject-position and predicate-position results must be DISJOINT (they
    // are different quads) — proving the single id is being sliced by
    // POSITION in the query, not conflated into one bucket.
    assert!(actual_p_subject.is_disjoint(&actual_p_predicate));

    // `(?, q, ?)` — q, a PURE predicate (never a subject/object anywhere),
    // used as a predicate constant. This is the exact shape that failed with
    // the earlier split-id-space dictionary (id_by_value(q) was None there).
    let expected_q_predicate = brute_force_pattern(&dataset, None, Some(&q_value), None);
    let actual_q_predicate = pack_pattern(&dict, &triples, None, Some(&q_value), None);
    assert_eq!(actual_q_predicate, expected_q_predicate);
    assert_eq!(
        actual_q_predicate.len(),
        2,
        "exactly two quads have q as predicate"
    );

    // q, having no subject/object role anywhere, must match NOTHING when used
    // as a subject or object constant.
    assert!(pack_pattern(&dict, &triples, Some(&q_value), None, None).is_empty());
    assert!(pack_pattern(&dict, &triples, None, None, Some(&q_value)).is_empty());
}
