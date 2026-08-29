// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Falsifiable equivalence tests for the subject-narrowed reifier lookup
//! [`DatasetView::reifier_quads_of`] across EVERY backend that overrides it.
//!
//! The trait's contract is exact: `reifier_quads_of(r)` must yield the same rows in
//! the same order as `reifier_quads().filter(|q| q.s == r)`. Three of the four
//! shipped views replace that filter with a sub-linear lookup that assumes their
//! reifier side table is sorted with the reifier as its primary key — an assumption
//! that, if wrong, returns WRONG ROWS rather than failing. These tests are the guard
//! on that assumption: they compare the narrowed lookup against the unnarrowed scan
//! it claims to be equivalent to, for every term in the dataset, on
//! [`RdfDataset`], `Arc<RdfDataset>`, `PackView`, `PagedDataset`, and the fallible
//! `PagedQueryView`.
//!
//! The corpus deliberately holds the shapes contiguity has to survive: several
//! reifiers binding ONE triple, one reifier binding SEVERAL triples, rows in the
//! default graph and in two named graphs, reifier terms interleaved with ordinary
//! terms in the term table, and (for the paged views) reifier rows for the SAME
//! reifier split across two pages. Bindings are pushed in an order matching neither
//! the frozen sort order nor the id order, so the runs are contiguous because of the
//! freeze sort, not by accident of push order.

use std::sync::Arc;

use purrdf_core::{
    DatasetView, InMemoryPageProvider, PackBuilder, PackView, PagedDataset, PagedQueryLimits,
    QuadIds, RdfDataset, RdfDatasetBuilder, TermId, TermRef, TermValue,
};

/// An `example.org` IRI value.
fn iri(name: &str) -> TermValue {
    TermValue::iri(format!("http://example.org/{name}"))
}

/// Resolve a view id to its dataset-INDEPENDENT [`TermValue`], recursing through a
/// literal's datatype and a triple term's components. Generic over any
/// [`DatasetView`] so the same routine reads every backend under test (they mint
/// unrelated id spaces, so raw ids are not comparable across them).
fn to_value<V: DatasetView>(v: &V, id: V::Id) -> TermValue {
    match v.resolve(id) {
        TermRef::Iri(s) => TermValue::iri(s),
        TermRef::Blank { label, scope } => TermValue::Blank {
            label: label.to_owned(),
            scope,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let datatype = match v.resolve(datatype) {
                TermRef::Iri(s) => s.to_owned(),
                other => panic!("literal datatype must resolve to an IRI, got {other:?}"),
            };
            TermValue::Literal {
                lexical_form: lexical.to_owned(),
                datatype,
                language: language.map(str::to_owned),
                direction,
            }
        }
        TermRef::Triple { s, p, o } => TermValue::Triple {
            s: Box::new(to_value(v, s)),
            p: Box::new(to_value(v, p)),
            o: Box::new(to_value(v, o)),
        },
    }
}

/// One virtual reifier quad rendered as dataset-independent values, so rows from two
/// different id spaces can be compared.
type ValueQuad = (TermValue, TermValue, TermValue, Option<TermValue>);

fn value_quad<V: DatasetView>(v: &V, q: QuadIds<V::Id>) -> ValueQuad {
    (
        to_value(v, q.s),
        to_value(v, q.p),
        to_value(v, q.o),
        q.g.map(|g| to_value(v, g)),
    )
}

/// The names every fixture interns, in interning order. The probe set below is
/// exactly this list, so it covers reifiers, non-reifiers, ids below the smallest
/// reifier id and ids above the largest.
const TERM_NAMES: [&str; 10] = [
    "p",
    "rA",
    "s1",
    "rB",
    "s2",
    "rC",
    "s3",
    "g1",
    "g2",
    "not-a-reifier",
];

/// The reifier bindings the fixtures push, as
/// `(reifier, triple-index, graph)` with `triple-index` selecting one of the three
/// triple terms and `graph` naming a fixture graph (`None` = default graph). The
/// order here is the PUSH order — deliberately neither sorted nor id-ordered.
const BINDINGS: [(&str, usize, Option<&str>); 9] = [
    ("rC", 1, Some("g2")),
    ("rA", 2, None),
    ("rB", 0, Some("g1")),
    ("rA", 0, Some("g2")),
    ("rC", 1, None),
    ("rA", 0, Some("g1")),
    ("rB", 0, None),
    ("rA", 1, None),
    ("rC", 0, Some("g1")),
];

/// Freeze one page holding the bindings selected by `keep` (an index filter over
/// [`BINDINGS`]), plus one base quad and one annotation per name in `anchors` so the
/// dataset is never side-table-only and the otherwise-unreferenced `not-a-reifier`
/// term survives into a pack dictionary / page translation. Every page interns
/// [`TERM_NAMES`] in the SAME order, so a term's local id is stable across pages
/// while the global/unified id spaces differ.
///
/// `anchors` must be DISJOINT between the pages of one `PagedDataset`: the paged seal
/// refuses overlapping rows in the primary and both side tables.
fn build_page(anchors: &[&str], keep: impl Fn(usize) -> bool) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let mut ids: Vec<TermId> = Vec::new();
    for name in TERM_NAMES {
        ids.push(b.intern_iri(&format!("http://example.org/{name}")));
    }
    let id_of = |name: &str| {
        let index = TERM_NAMES
            .iter()
            .position(|n| *n == name)
            .expect("fixture term name");
        ids[index]
    };
    let (p, s1, s2, s3) = (id_of("p"), id_of("s1"), id_of("s2"), id_of("s3"));
    let triples = [
        b.intern_triple(s1, p, s2),
        b.intern_triple(s2, p, s3),
        b.intern_triple(s3, p, s1),
    ];

    for (index, (reifier, triple, graph)) in BINDINGS.iter().enumerate() {
        if !keep(index) {
            continue;
        }
        b.push_reifier_in_graph(id_of(reifier), triples[*triple], graph.map(&id_of));
    }
    for anchor in anchors {
        b.push_quad(id_of("not-a-reifier"), p, id_of(anchor), None);
        b.push_annotation(id_of("rB"), p, id_of(anchor));
    }
    b.freeze().expect("valid reifier fixture")
}

/// The single-dataset fixture: every binding in one frozen `RdfDataset`.
fn build_fixture() -> Arc<RdfDataset> {
    build_page(&["s1", "s2"], |_| true)
}

/// The two-page split of the same corpus. The cut at index 5 puts SOME of every
/// reifier's bindings on each page, so each reifier's rows straddle the page boundary
/// — the composition the paged override has to keep in page order.
fn build_split_pages() -> Vec<Arc<RdfDataset>> {
    vec![
        build_page(&["s1"], |i| i < 5),
        build_page(&["s2"], |i| i >= 5),
    ]
}

/// Assert the trait's exact contract on `view`: for EVERY term the fixture interns,
/// the narrowed lookup yields the same rows in the same order as the filtered full
/// scan, and the per-reifier runs together partition the whole reifier table.
///
/// The comparison is by resolved value on both sides, so it reads identically on a
/// view with its own id space; the ORDER is compared, not just the set.
fn assert_narrowed_equals_scan<V: DatasetView>(view: &V, label: &str) {
    let total = view.reifier_quads().count();
    assert!(
        total >= 9,
        "{label}: fixture must hold enough reifier rows for contiguity to matter (got {total})"
    );

    let mut reached = 0usize;
    for name in TERM_NAMES {
        let Some(probe) = view.term_id_by_value(&iri(name)) else {
            continue;
        };
        let expected: Vec<ValueQuad> = view
            .reifier_quads()
            .filter(|q| q.s == probe)
            .map(|q| value_quad(view, q))
            .collect();
        let actual: Vec<ValueQuad> = view
            .reifier_quads_of(probe)
            .map(|q| value_quad(view, q))
            .collect();
        assert_eq!(
            actual, expected,
            "{label}: reifier_quads_of({name}) diverged from reifier_quads().filter(s == {name})"
        );
        reached += actual.len();
    }
    // Every reifier row's subject is one of the probed terms, and each row has exactly
    // one subject, so the runs must partition the table. A row stranded outside its own
    // reifier's run — precisely what a wrong sort-key assumption produces — shows up
    // here as a short total.
    assert_eq!(
        reached, total,
        "{label}: the per-reifier runs must partition the whole reifier table"
    );

    // A term that IS interned but reifies nothing yields nothing: the run is empty and
    // the search lands mid-table rather than at either end.
    let bystander = view
        .term_id_by_value(&iri("not-a-reifier"))
        .expect("bystander is interned");
    assert_eq!(
        view.reifier_quads_of(bystander).count(),
        0,
        "{label}: a non-reifier term must address an empty run"
    );
}

#[test]
fn rdf_dataset_narrowed_lookup_equals_the_scan() {
    let ds = build_fixture();
    assert_narrowed_equals_scan(&*ds, "RdfDataset");
    // The `Arc` blanket impl must forward the override, not fall back to the default.
    assert_narrowed_equals_scan(&ds, "Arc<RdfDataset>");
}

#[test]
fn pack_view_narrowed_lookup_equals_the_scan() {
    let ds = build_fixture();
    let bytes = PackBuilder::build_bytes(&ds).expect("pack build");
    let pack = PackView::from_bytes(&bytes).expect("pack opens");
    assert_narrowed_equals_scan(&pack, "PackView");
}

#[test]
fn paged_dataset_narrowed_lookup_equals_the_scan() {
    let paged =
        PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(build_split_pages())))
            .expect("pages seal (quad-disjoint side tables)");
    assert_narrowed_equals_scan(&paged, "PagedDataset");
}

#[test]
fn paged_query_view_narrowed_lookup_equals_the_scan() {
    let paged =
        PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(build_split_pages())))
            .expect("pages seal (quad-disjoint side tables)");
    // Limits generous enough that no probe trips the operation budget: a budget stop
    // would make both sides of the comparison empty and the test vacuous.
    let view = paged.query_view(PagedQueryLimits::new(u64::MAX, u64::MAX));
    assert_narrowed_equals_scan(&view, "PagedQueryView");
}

#[test]
fn paged_dataset_composes_one_reifiers_rows_across_pages() {
    // An independent, hand-written expectation (not derived from `reifier_quads`) that
    // the paged override really does reach BOTH pages for one reifier: `rA` owns four
    // bindings, which the cut at index 5 puts two-and-two on the two pages.
    let paged =
        PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(build_split_pages())))
            .expect("pages seal");
    let r_a = paged.term_id_by_value(&iri("rA")).expect("rA interned");
    let mut rows: Vec<ValueQuad> = paged
        .reifier_quads_of(r_a)
        .map(|q| value_quad(&paged, q))
        .collect();
    rows.sort_by_key(|row| format!("{row:?}"));
    assert_eq!(rows.len(), 4, "rA owns four bindings across the two pages");
    for (_, p, _, _) in &rows {
        assert_eq!(
            *p,
            TermValue::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies")
        );
    }
}
