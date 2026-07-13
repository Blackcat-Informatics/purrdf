// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Falsifiable parity tests for the `PackView`-over-`DatasetView` seam (Task 6 of
//! the succinct-pack-codec feature): every read-side query the trait exposes must
//! answer IDENTICALLY (by resolved value, not by raw id — the two views mint
//! unrelated id spaces) whether it runs over the reference `RdfDataset` or over a
//! `PackView` opened from `PackBuilder::build_bytes(&dataset)`'s output.
//!
//! The fixture corpus deliberately exercises every seam the pack codec's unified,
//! single-id-space dictionary claims to unify: a term used as both predicate AND
//! subject (`ex:knows`), an RDF 1.2 triple term used as both subject AND object,
//! `>= 2` named graphs plus the default graph, every literal shape (simple, typed,
//! language-tagged, directional), a scoped blank node, an `rdf:List` collection, and
//! reifier + annotation side-table rows (including a graph-scoped annotation).

use std::sync::Arc;

use purrdf_core::{
    BlankScope, DatasetView, GraphMatch, PackBuilder, PackView, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, RdfStoreCapabilities, RdfTextDirection, TermRef, TermValue,
};

// Well-known RDF Collection vocabulary IRIs (crate-internal constants are not
// public; these are the standard IRIs, mirroring `tests/paged_backend.rs`).
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

/// An `example.org` IRI value.
fn iri(name: &str) -> TermValue {
    TermValue::iri(format!("http://example.org/{name}"))
}

/// Resolve a view id to its dataset-INDEPENDENT `TermValue`, recursing through a
/// literal's datatype and a triple term's components. Generic over any
/// `DatasetView`, so the SAME routine reads the reference `RdfDataset` and the
/// `PackView` under test and lets their rows be compared by value (the two mint
/// unrelated id spaces, so comparing raw ids would be meaningless).
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

/// A deterministic sort key for a value row (`TermValue` is not `Ord`; its `Debug`
/// form is total and dataset-independent).
fn row_key(row: &[TermValue]) -> String {
    format!("{row:?}")
}

/// Collect every quad of the view as sorted `[s, p, o, g?]` value rows via the
/// generic trait surface only. `g` is rendered as an extra element (a sentinel for
/// the default graph) so named-graph quads stay distinguishable — mirrors
/// `tests/paged_backend.rs`'s `collect_rows`.
fn collect_rows<V: DatasetView>(v: &V) -> Vec<Vec<TermValue>> {
    let mut rows: Vec<Vec<TermValue>> = v
        .quads_for_pattern(None, None, None, GraphMatch::Any)
        .map(|q| {
            let mut row = vec![to_value(v, q.s), to_value(v, q.p), to_value(v, q.o)];
            row.push(q.g.map_or_else(|| TermValue::iri("urn:default-graph"), |g| to_value(v, g)));
            row
        })
        .collect();
    rows.sort_by_key(|r| row_key(r));
    rows
}

/// A value-keyed graph filter — the write-agnostic twin of `GraphMatch<V::Id>` a
/// test case can name without committing to either view's id space.
enum GraphSpec {
    Any,
    Default,
    Named(TermValue),
}

/// Resolve a `GraphSpec` into `V`'s own `GraphMatch<V::Id>`, exactly as the SPARQL
/// evaluator resolves a graph term via `term_id_by_value` before probing.
fn resolve_graph<V: DatasetView>(v: &V, spec: &GraphSpec) -> GraphMatch<V::Id> {
    match spec {
        GraphSpec::Any => GraphMatch::Any,
        GraphSpec::Default => GraphMatch::Default,
        GraphSpec::Named(value) => {
            let id = v
                .term_id_by_value(value)
                .unwrap_or_else(|| panic!("graph value {value:?} must be interned"));
            GraphMatch::Named(id)
        }
    }
}

/// Resolve an optional bound `TermValue` to `V`'s id via `term_id_by_value` —
/// EXACTLY how the evaluator resolves a pattern's bound constants before probing
/// (never by minting).
fn opt_id<V: DatasetView>(v: &V, value: Option<&TermValue>) -> Option<V::Id> {
    let value = value?;
    Some(
        v.term_id_by_value(value)
            .unwrap_or_else(|| panic!("value {value:?} must be interned")),
    )
}

/// Run one `(s, p, o, g)` pattern query over `v` and collect the sorted `[s, p, o,
/// g?]` value rows — the generic pattern-query probe both views are compared
/// through.
fn pattern_rows<V: DatasetView>(
    v: &V,
    s: Option<&TermValue>,
    p: Option<&TermValue>,
    o: Option<&TermValue>,
    g: &GraphSpec,
) -> Vec<Vec<TermValue>> {
    let s_id = opt_id(v, s);
    let p_id = opt_id(v, p);
    let o_id = opt_id(v, o);
    let g_match = resolve_graph(v, g);
    let mut rows: Vec<Vec<TermValue>> = v
        .quads_for_pattern(s_id, p_id, o_id, g_match)
        .map(|q| {
            let mut row = vec![to_value(v, q.s), to_value(v, q.p), to_value(v, q.o)];
            row.push(q.g.map_or_else(|| TermValue::iri("urn:default-graph"), |g| to_value(v, g)));
            row
        })
        .collect();
    rows.sort_by_key(|r| row_key(r));
    rows
}

/// Generic list walk: resolve `head` by value, then `DatasetView::members`, mapping
/// each member id back to its value.
fn walk_members<V: DatasetView>(v: &V, head: &TermValue, graph: &GraphSpec) -> Vec<TermValue> {
    let head_id = v
        .term_id_by_value(head)
        .unwrap_or_else(|| panic!("list head {head:?} must be interned"));
    let g_match = resolve_graph(v, graph);
    v.members(head_id, g_match)
        .expect("well-formed list")
        .into_iter()
        .map(|id| to_value(v, id))
        .collect()
}

/// Generic reifier/annotation/named-graph collectors, sorted by their `Debug` form
/// for order-independent comparison.
fn reifier_rows<V: DatasetView>(v: &V) -> Vec<Vec<TermValue>> {
    let mut rows: Vec<Vec<TermValue>> = v
        .reifier_quads()
        .map(|q| {
            vec![
                to_value(v, q.s),
                to_value(v, q.p),
                to_value(v, q.o),
                q.g.map_or_else(|| TermValue::iri("urn:default-graph"), |g| to_value(v, g)),
            ]
        })
        .collect();
    rows.sort_by_key(|r| row_key(r));
    rows
}

fn annotation_rows<V: DatasetView>(v: &V) -> Vec<Vec<TermValue>> {
    let mut rows: Vec<Vec<TermValue>> = v
        .annotation_quads()
        .map(|q| {
            vec![
                to_value(v, q.s),
                to_value(v, q.p),
                to_value(v, q.o),
                q.g.map_or_else(|| TermValue::iri("urn:default-graph"), |g| to_value(v, g)),
            ]
        })
        .collect();
    rows.sort_by_key(|r| row_key(r));
    rows
}

fn annotations_of_rows<V: DatasetView>(v: &V, reifier: &TermValue) -> Vec<Vec<TermValue>> {
    let reifier_id = v
        .term_id_by_value(reifier)
        .unwrap_or_else(|| panic!("reifier {reifier:?} must be interned"));
    let mut rows: Vec<Vec<TermValue>> = v
        .annotations_of_with_graph(reifier_id)
        .map(|(p, o, g)| {
            vec![
                to_value(v, p),
                to_value(v, o),
                g.map_or_else(|| TermValue::iri("urn:default-graph"), |g| to_value(v, g)),
            ]
        })
        .collect();
    rows.sort_by_key(|r| row_key(r));
    rows
}

fn named_graph_values<V: DatasetView>(v: &V) -> Vec<TermValue> {
    let mut values: Vec<TermValue> = v.named_graphs().map(|g| to_value(v, g)).collect();
    values.sort_by_key(|v| format!("{v:?}"));
    values
}

// ── The fixture corpus ──────────────────────────────────────────────────────────

/// Build the rich fixture `RdfDataset`: default graph + 2 named graphs, every
/// literal shape, a scoped blank node, a triple term used as subject AND object, a
/// term (`ex:knows`) used as both predicate and subject, an `rdf:List` collection,
/// and reifier + annotation side-table rows (one annotation graph-scoped).
fn build_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();

    let alice = b.intern_iri("http://example.org/alice");
    let bob = b.intern_iri("http://example.org/bob");
    let carol = b.intern_iri("http://example.org/carol");
    let knows = b.intern_iri("http://example.org/knows");
    let age = b.intern_iri("http://example.org/age");
    let name = b.intern_iri("http://example.org/name");
    let sees = b.intern_iri("http://example.org/sees");
    let likes = b.intern_iri("http://example.org/likes");
    let greeting = b.intern_iri("http://example.org/greeting");
    let states_fact = b.intern_iri("http://example.org/statesFact");
    let certainty = b.intern_iri("http://example.org/certainty");
    let confidence = b.intern_iri("http://example.org/confidence");
    let high = b.intern_iri("http://example.org/high");
    let source = b.intern_iri("http://example.org/source");
    let doc = b.intern_iri("http://example.org/doc");
    let meta = b.intern_iri("http://example.org/meta");
    let reifier = b.intern_iri("http://example.org/r");
    let graph1 = b.intern_iri("http://example.org/graph1");
    let graph2 = b.intern_iri("http://example.org/graph2");

    let c1 = b.intern_iri("http://example.org/c1");
    let c2 = b.intern_iri("http://example.org/c2");
    let c3 = b.intern_iri("http://example.org/c3");
    let item1 = b.intern_iri("http://example.org/item1");
    let item2 = b.intern_iri("http://example.org/item2");
    let item3 = b.intern_iri("http://example.org/item3");
    let rdf_first = b.intern_iri(RDF_FIRST);
    let rdf_rest = b.intern_iri(RDF_REST);
    let rdf_nil = b.intern_iri(RDF_NIL);

    let blank = b.intern_blank("b1", BlankScope::DEFAULT);

    let forty_two = b.intern_literal(RdfLiteral {
        lexical_form: "42".to_string(),
        datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_string()),
        language: None,
        direction: None,
    });
    let alice_name_en = b.intern_literal(RdfLiteral {
        lexical_form: "Alice".to_string(),
        datatype: None,
        language: Some("en".to_string()),
        direction: None,
    });
    let anon_name = b.intern_literal(RdfLiteral {
        lexical_form: "Anon".to_string(),
        datatype: None,
        language: None,
        direction: None,
    });
    let knows_label = b.intern_literal(RdfLiteral {
        lexical_form: "the knows predicate".to_string(),
        datatype: None,
        language: None,
        direction: None,
    });
    let hello_ltr = b.intern_literal(RdfLiteral {
        lexical_form: "Hello".to_string(),
        datatype: None,
        language: Some("en".to_string()),
        direction: Some(RdfTextDirection::Ltr),
    });
    let carol_age = b.intern_literal(RdfLiteral {
        lexical_form: "30".to_string(),
        datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_string()),
        language: None,
        direction: None,
    });

    // The RDF 1.2 triple term `<<(alice knows bob)>>`, used as the OBJECT of an
    // asserted quad (`meta statesFact <<...>>`) below, AND as the SUBJECT of a
    // NESTED triple term `<<<<(alice knows bob)>> certainty "high">>` (a component
    // position — legal per `require_triple_component_subject`, unlike an asserted
    // quad's top-level subject, which may never be a triple term). The nested term
    // is itself asserted as the object of a second quad, so `alice_knows_bob`
    // genuinely occupies both roles.
    let alice_knows_bob = b.intern_triple(alice, knows, bob);
    let annotated_alice_knows_bob = b.intern_triple(alice_knows_bob, certainty, high);

    // -- Default graph --------------------------------------------------------
    b.push_quad(alice, knows, bob, None);
    b.push_quad(alice, age, forty_two, None);
    b.push_quad(alice, name, alice_name_en, None);
    // `knows` used as a SUBJECT here, and as a PREDICATE above — the unified-id
    // seam the pack dictionary's single-id-space model exists to prove.
    b.push_quad(knows, name, knows_label, None);
    b.push_quad(blank, name, anon_name, None);
    b.push_quad(bob, sees, blank, None);
    b.push_quad(alice, greeting, hello_ltr, None);
    b.push_quad(meta, states_fact, alice_knows_bob, None);
    b.push_quad(meta, states_fact, annotated_alice_knows_bob, None);
    // An `rdf:List` collection `c1 -> c2 -> c3 -> nil`.
    b.push_quad(c1, rdf_first, item1, None);
    b.push_quad(c1, rdf_rest, c2, None);
    b.push_quad(c2, rdf_first, item2, None);
    b.push_quad(c2, rdf_rest, c3, None);
    b.push_quad(c3, rdf_first, item3, None);
    b.push_quad(c3, rdf_rest, rdf_nil, None);

    // -- Named graph 1 ----------------------------------------------------------
    b.push_quad(bob, knows, carol, Some(graph1));
    b.push_quad(carol, age, carol_age, Some(graph1));

    // -- Named graph 2 ----------------------------------------------------------
    b.push_quad(carol, likes, alice, Some(graph2));

    // -- Reifier + annotations ---------------------------------------------------
    b.push_reifier(reifier, alice_knows_bob);
    b.push_annotation(reifier, confidence, high);
    // A second, graph-scoped annotation on the SAME reifier — exercises the graph
    // slot in `annotation_quads`/`annotations_of_with_graph`.
    b.push_annotation_in_graph(reifier, source, doc, Some(graph1));

    b.freeze().expect("fixture dataset must validate")
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// Build the pack bytes for `dataset` and open a `PackView` over them — the
/// standard fixture-to-pack path every test below shares.
fn build_pack_bytes(dataset: &RdfDataset) -> Vec<u8> {
    PackBuilder::build_bytes(dataset).expect("pack build must succeed for a well-formed fixture")
}

#[test]
fn quads_set_equal_source() {
    let single = build_fixture();
    let bytes = build_pack_bytes(&single);
    let pack = PackView::from_bytes(&bytes).expect("pack opens");

    assert_eq!(
        collect_rows(&*single),
        collect_rows(&pack),
        "PackView's whole-dataset scan must equal the source RdfDataset's, by value"
    );
    // Sanity: the corpus is non-trivial.
    assert!(collect_rows(&pack).len() >= 15);
}

#[test]
fn pattern_shapes_set_equal_source_including_predicate_that_is_also_subject() {
    let single = build_fixture();
    let bytes = build_pack_bytes(&single);
    let pack = PackView::from_bytes(&bytes).expect("pack opens");

    let alice = iri("alice");
    let bob = iri("bob");
    let knows = iri("knows");
    let carol = iri("carol");
    let graph1 = iri("graph1");
    let graph2 = iri("graph2");

    #[allow(clippy::type_complexity)]
    let cases: Vec<(
        Option<TermValue>,
        Option<TermValue>,
        Option<TermValue>,
        GraphSpec,
        &str,
    )> = vec![
        (None, None, None, GraphSpec::Any, "whole scan, any graph"),
        (
            Some(alice.clone()),
            None,
            None,
            GraphSpec::Any,
            "subject bound",
        ),
        (
            None,
            // `knows` is bound here as a PREDICATE constant even though it is ALSO
            // used as a SUBJECT elsewhere in the corpus (`knows name "the knows
            // predicate"`) — proves the unified single-id-space seam: resolving
            // `knows` via `term_id_by_value` yields ONE id usable in either role.
            Some(knows.clone()),
            None,
            GraphSpec::Any,
            "predicate bound to a term that is also used as a subject",
        ),
        (
            None,
            None,
            Some(bob.clone()),
            GraphSpec::Any,
            "object bound",
        ),
        (
            Some(alice.clone()),
            Some(knows.clone()),
            None,
            GraphSpec::Any,
            "subject+predicate bound",
        ),
        (
            Some(alice),
            Some(knows.clone()),
            Some(bob),
            GraphSpec::Any,
            "fully bound",
        ),
        (None, None, None, GraphSpec::Default, "default graph only"),
        (
            None,
            None,
            None,
            GraphSpec::Named(graph1.clone()),
            "named graph 1 only",
        ),
        (
            Some(carol),
            None,
            None,
            GraphSpec::Named(graph2),
            "subject bound in named graph 2",
        ),
        (
            None,
            Some(knows),
            None,
            GraphSpec::Named(graph1),
            "predicate bound in named graph 1 (also proves cross-graph predicate reuse)",
        ),
    ];

    for (s, p, o, g, description) in &cases {
        let single_rows = pattern_rows(&*single, s.as_ref(), p.as_ref(), o.as_ref(), g);
        let pack_rows = pattern_rows(&pack, s.as_ref(), p.as_ref(), o.as_ref(), g);
        assert_eq!(
            single_rows, pack_rows,
            "pattern parity failed for case: {description}"
        );
    }
}

#[test]
fn resolve_and_quad_refs_match_source() {
    let single = build_fixture();
    let bytes = build_pack_bytes(&single);
    let pack = PackView::from_bytes(&bytes).expect("pack opens");

    // `resolve` parity: every value the source view can resolve is reachable and
    // resolves identically through the pack.
    let single_values: Vec<TermValue> = single
        .quads()
        .flat_map(|q| {
            let mut vals = vec![
                to_value(&*single, q.s),
                to_value(&*single, q.p),
                to_value(&*single, q.o),
            ];
            if let Some(g) = q.g {
                vals.push(to_value(&*single, g));
            }
            vals
        })
        .collect();
    for value in &single_values {
        let pack_id = pack
            .term_id_by_value(value)
            .unwrap_or_else(|| panic!("value {value:?} must be interned in the pack"));
        assert_eq!(
            &to_value(&pack, pack_id),
            value,
            "resolve parity for {value:?}"
        );
    }

    // `quad_refs` parity: resolved quad rows (via the borrowed `QuadRef` path) match
    // by value, set-equal.
    fn quad_ref_rows<V: DatasetView>(v: &V) -> Vec<Vec<TermValue>> {
        v.quad_refs()
            .map(|qr| {
                let s = quad_ref_term_value(v, qr.s);
                let p = quad_ref_term_value(v, qr.p);
                let o = quad_ref_term_value(v, qr.o);
                let mut row = vec![s, p, o];
                row.push(qr.g.map_or_else(
                    || TermValue::iri("urn:default-graph"),
                    |g| quad_ref_term_value(v, g),
                ));
                row
            })
            .collect()
    }
    fn quad_ref_term_value<V: DatasetView>(v: &V, t: TermRef<'_, V::Id>) -> TermValue {
        match t {
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

    let mut single_rows = quad_ref_rows(&*single);
    let mut pack_rows = quad_ref_rows(&pack);
    single_rows.sort_by_key(|r| row_key(r));
    pack_rows.sort_by_key(|r| row_key(r));
    assert_eq!(single_rows, pack_rows, "quad_refs parity");
}

#[test]
fn side_table_views_set_equal_source() {
    let single = build_fixture();
    let bytes = build_pack_bytes(&single);
    let pack = PackView::from_bytes(&bytes).expect("pack opens");

    assert_eq!(
        reifier_rows(&*single),
        reifier_rows(&pack),
        "reifier_quads parity"
    );
    assert_eq!(
        annotation_rows(&*single),
        annotation_rows(&pack),
        "annotation_quads parity"
    );
    assert_eq!(
        annotations_of_rows(&*single, &iri("r")),
        annotations_of_rows(&pack, &iri("r")),
        "annotations_of_with_graph parity"
    );
    assert_eq!(
        named_graph_values(&*single),
        named_graph_values(&pack),
        "named_graphs parity"
    );
    // Falsifiability: the fixture actually carries side-table rows and 2 named
    // graphs, so an empty-default bug would be caught.
    assert_eq!(reifier_rows(&pack).len(), 1);
    assert_eq!(annotation_rows(&pack).len(), 2);
    assert_eq!(named_graph_values(&pack).len(), 2);
}

#[test]
fn collection_members_match_source() {
    let single = build_fixture();
    let bytes = build_pack_bytes(&single);
    let pack = PackView::from_bytes(&bytes).expect("pack opens");

    let single_members = walk_members(&*single, &iri("c1"), &GraphSpec::Default);
    let pack_members = walk_members(&pack, &iri("c1"), &GraphSpec::Default);
    assert_eq!(
        single_members,
        vec![iri("item1"), iri("item2"), iri("item3")],
        "list members in order"
    );
    assert_eq!(single_members, pack_members, "member parity");

    // `rdf_list` directly (not just the `members` dispatcher) agrees too.
    let head_id_single = single.term_id_by_value(&iri("c1")).expect("c1 interned");
    let head_id_pack = pack.term_id_by_value(&iri("c1")).expect("c1 interned");
    let single_list = single
        .rdf_list(head_id_single, GraphMatch::Default)
        .expect("well-formed list");
    let pack_list = pack
        .rdf_list(head_id_pack, GraphMatch::Default)
        .expect("well-formed list");
    let single_list_values: Vec<TermValue> = single_list
        .into_iter()
        .map(|id| to_value(&*single, id))
        .collect();
    let pack_list_values: Vec<TermValue> = pack_list
        .into_iter()
        .map(|id| to_value(&pack, id))
        .collect();
    assert_eq!(single_list_values, pack_list_values, "rdf_list parity");
}

#[test]
fn term_id_by_value_absent_capabilities_and_term_count() {
    let single = build_fixture();
    let bytes = build_pack_bytes(&single);
    let pack = PackView::from_bytes(&bytes).expect("pack opens");

    // Absence is an empty match, never an error.
    let absent = iri("this-value-was-never-interned");
    assert_eq!(single.term_id_by_value(&absent), None);
    assert_eq!(pack.term_id_by_value(&absent), None);

    // Capabilities agree: named graphs, quoted triples, reifiers, and annotations
    // are all exercised by the fixture; source locations/loss records/lookaside are
    // not.
    let single_caps = DatasetView::capabilities(&*single);
    let pack_caps = pack.capabilities();
    assert_eq!(single_caps, pack_caps, "capabilities parity");
    assert_eq!(
        pack_caps,
        RdfStoreCapabilities {
            named_graphs: true,
            quoted_triples: true,
            reifiers: true,
            annotations: true,
            source_locations: false,
            loss_records: false,
            lookaside: false,
        }
    );

    // `term_count` matches (the pack dictionary's closure may mint a couple of
    // extra structural terms — e.g. `rdf:reifies` — beyond the source's own term
    // table, so this asserts the pack sees at least every source term, and that
    // every source term round-trips through the pack, rather than a raw count
    // equality that would be sensitive to that closure).
    assert!(pack.term_count() >= single.term_count());
    for value in &named_graph_values(&*single) {
        assert!(pack.term_id_by_value(value).is_some());
    }
}

/// The compile-time bound Task 6 requires: `PackView` plugs into any generic
/// `DatasetView` consumer (the SPARQL evaluator, in particular) and is
/// thread-shareable.
#[test]
fn pack_view_is_dataset_view_send_sync() {
    fn assert_dataset_view<T: DatasetView>() {}
    fn assert_send_sync<T: Send + Sync>() {}
    assert_dataset_view::<PackView<'static>>();
    assert_send_sync::<PackView<'static>>();

    // And a live instance actually behaves as one (not just type-checks).
    let single = build_fixture();
    let bytes = build_pack_bytes(&single);
    let pack = PackView::from_bytes(&bytes).expect("pack opens");
    fn use_as_dataset_view<V: DatasetView>(v: &V) -> usize {
        v.term_count()
    }
    assert!(use_as_dataset_view(&pack) > 0);
}
