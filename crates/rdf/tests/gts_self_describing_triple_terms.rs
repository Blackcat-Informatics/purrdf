// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A self-describing quoted triple must survive the container read path.
//!
//! `purrdf_gts::model::Term` states a quoted triple's components in its own `triple`
//! slot (wire `"tt"`). That is the MODERN spelling and the only one that can express
//! RDF 1.2 faithfully — `rdf:reifies` is not functional, so one reifier id may bind
//! several distinct triples and a triple TERM cannot borrow its identity from one —
//! and it is what `gts_write::to_gts` emits for every quoted triple.
//!
//! The first-party `SerGraph` the codecs and the IR fold consume has no such slot: it
//! spells a triple term as the SELF-REIFIER sentinel, the term's `reifier` being its
//! own id. The bridge between the two models must therefore TRANSLATE. When it did
//! not, a `tt`-only term arrived carrying no components at all: it rendered as the
//! fabricated blank node `_:unbound_triple_N` on the text path and hard-failed the IR
//! fold with `native-codec-unbound-triple-term`. Silent loss on the newest path.
//!
//! These tests drive the public container surface end to end and assert the quoted
//! triple comes out as a quoted triple.

use purrdf_gts::model::{Graph, Term, TermKind};
use purrdf_rdf::gts::{
    dataset_from_gts_graph, flattened_dataset_from_bytes, flattened_dataset_from_gts_graph,
};
use purrdf_rdf::gts_write::to_gts;
use purrdf_rdf::ir::{RdfDataset, RdfDatasetBuilder};
use purrdf_rdf::{RdfLookaside, SerializeGraph, TermRef, serialize_dataset};
use std::sync::Arc;

const S: &str = "https://example.org/s";
const P: &str = "https://example.org/p";
const O: &str = "https://example.org/o";
const SAYS: &str = "https://example.org/says";

fn iri(value: &str) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.to_owned()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    }
}

/// The modern spelling: components in `tt`, no reifier, no statement-layer row.
fn tt_only_triple_term(spo: (usize, usize, usize)) -> Term {
    Term {
        kind: TermKind::Triple,
        value: Some(String::new()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: Some(spo),
    }
}

/// `<s> <says> <<( <s> <p> <o> )>>` with the quoted triple in the `tt` spelling.
fn tt_only_graph() -> Graph {
    let mut graph = Graph::default();
    graph.terms.push(iri(S)); // 0
    graph.terms.push(iri(P)); // 1
    graph.terms.push(iri(O)); // 2
    graph.terms.push(iri(SAYS)); // 3
    graph.terms.push(tt_only_triple_term((0, 1, 2))); // 4
    graph.quads.push((0, 3, 4, None));
    graph.segment_profiles.push("rdf12".to_owned());
    graph
}

fn nquads(dataset: &RdfDataset) -> String {
    String::from_utf8(
        serialize_dataset(dataset, "application/n-quads", SerializeGraph::Dataset)
            .expect("serialize to N-Quads"),
    )
    .expect("N-Quads is UTF-8")
}

/// The object of the one quad is the quoted triple itself, resolved to its three
/// components — not a blank node, and not an error.
fn assert_quoted_triple_object(dataset: &RdfDataset) {
    let quads: Vec<_> = dataset.quads().collect();
    assert_eq!(quads.len(), 1, "the single source quad survives");
    let TermRef::Triple { s, p, o } = dataset.resolve(quads[0].o) else {
        panic!(
            "the object must be a triple term, got {:?}",
            dataset.resolve(quads[0].o)
        );
    };
    assert_eq!(dataset.resolve(s), TermRef::Iri(S));
    assert_eq!(dataset.resolve(p), TermRef::Iri(P));
    assert_eq!(dataset.resolve(o), TermRef::Iri(O));
}

/// The bare bridge: a hand-built graph in the `tt` spelling folds into a dataset whose
/// object is the quoted triple, and serializes as `<<( s p o )>>`.
#[test]
fn a_tt_only_triple_term_folds_and_renders_as_a_quoted_triple() {
    let graph = tt_only_graph();

    let dataset = dataset_from_gts_graph(&graph).expect("a `tt`-only triple term folds");
    assert_quoted_triple_object(&dataset);

    let text = nquads(&dataset);
    assert_eq!(
        text,
        format!("<{S}> <{SAYS}> <<( <{S}> <{P}> <{O}> )>> .\n"),
        "the quoted triple renders inline, with no fabricated blank node"
    );
    assert!(
        !text.contains("unbound_triple"),
        "no `_:unbound_triple_N` placeholder may appear: {text}"
    );
    assert!(
        !text.contains("rdf-syntax-ns#reifies"),
        "a self-bound triple term is not an `rdf:reifies` statement: {text}"
    );
}

/// The flattening twin of the load path takes the same translation.
#[test]
fn the_flattened_load_path_carries_a_tt_only_triple_term() {
    let graph = tt_only_graph();
    let dataset =
        flattened_dataset_from_gts_graph(&graph).expect("a `tt`-only triple term folds flattened");
    assert_quoted_triple_object(&dataset);
}

/// The nesting case, which is where borrowing a reifier id could never have worked:
/// `<<( s says <<( s p o )>> )>>`. Both terms are `tt`-only, so both need translating,
/// and the inner one has to be resolvable while the outer one is being resolved.
#[test]
fn nested_tt_only_triple_terms_survive_to_their_leaves() {
    let mut graph = Graph::default();
    graph.terms.push(iri(S)); // 0
    graph.terms.push(iri(P)); // 1
    graph.terms.push(iri(O)); // 2
    graph.terms.push(iri(SAYS)); // 3
    graph.terms.push(tt_only_triple_term((0, 1, 2))); // 4 — inner
    graph.terms.push(tt_only_triple_term((0, 3, 4))); // 5 — outer
    graph.quads.push((0, 3, 5, None));

    let dataset = dataset_from_gts_graph(&graph).expect("nested `tt`-only triple terms fold");
    assert_eq!(
        nquads(&dataset),
        format!("<{S}> <{SAYS}> <<( <{S}> <{SAYS}> <<( <{S}> <{P}> <{O}> )>> )>> .\n"),
        "both levels of the nesting survive"
    );
}

/// The whole container round trip, through the bytes: an IR dataset holding a quoted
/// triple → `to_gts` (which writes the `tt` spelling and NO reifier) → the reader →
/// the bridge → an IR dataset again. This is the path a consumer actually takes, and
/// the one the dropped `tt` silently corrupted.
#[test]
fn a_quoted_triple_round_trips_through_gts_bytes() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(S);
    let p = builder.intern_iri(P);
    let o = builder.intern_iri(O);
    let says = builder.intern_iri(SAYS);
    let quoted = builder.intern_triple(s, p, o);
    builder.push_quad(s, says, quoted, None);
    let source: Arc<RdfDataset> = builder.freeze().expect("a valid source dataset");

    let bytes = to_gts(&source, &RdfLookaside::default(), "rdf12").expect("write GTS bytes");
    let restored = flattened_dataset_from_bytes(&bytes).expect("read the bytes back");

    assert_quoted_triple_object(&restored);
    assert_eq!(
        nquads(&restored),
        nquads(&source),
        "the round trip is text-identical to the source dataset"
    );
}

/// The translation must not disturb the ORIGINAL indirect spelling, which is still
/// legal on the wire: a triple term naming a reifier id whose statement-layer row
/// supplies the components. That row is a real `rdf:reifies` statement and must still
/// be emitted as one.
#[test]
fn the_indirect_reifier_spelling_is_left_alone() {
    let mut graph = Graph::default();
    graph.terms.push(iri(S)); // 0
    graph.terms.push(iri(P)); // 1
    graph.terms.push(iri(O)); // 2
    graph.terms.push(iri("https://example.org/r")); // 3 — the reifier
    graph.terms.push(Term {
        kind: TermKind::Triple,
        value: Some(String::new()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: Some(3),
        triple: None,
    }); // 4
    graph.reifiers.push((3, (0, 1, 2), None));
    graph.quads.push((0, 1, 2, None));

    let dataset = dataset_from_gts_graph(&graph).expect("the indirect spelling folds");
    let text = nquads(&dataset);
    assert!(
        text.contains(&format!(
            "<https://example.org/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> \
             <<( <{S}> <{P}> <{O}> )>>"
        )),
        "the reifier declaration is still emitted as an `rdf:reifies` row: {text}"
    );
}
