// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A SPARQL query is its own blank-node document.
//!
//! A `cdt:List` / `cdt:Map` lexical form is not opaque text: its
//! `BLANK_NODE_LABEL` tokens denote blank nodes of the document that wrote them
//! (see `purrdf_core::cdt_blank`). Query text is such a document, and it is a
//! DIFFERENT one from the dataset being queried — so
//! `BIND("[_:b, 42]"^^cdt:List AS ?l)` names a node distinct from the `_:b` of the
//! data, exactly as two Turtle files that both write `_:b` name two nodes.
//!
//! The upstream SEP-0009 corpus states this as `bnodes-turtle-sparql-01`…`-04`
//! (`vectors/sparql-cdt/bnodes/`). Those cases run against a dataset the
//! conformance harness assembles by MERGING source documents, which standardizes
//! every source apart from the default scope — so they would stay green even if
//! the engine collapsed the query into whatever scope its data happened to use.
//! Every dataset here is therefore built the way a plain single-document load
//! builds one: blank nodes at `BlankScope::DEFAULT`, the scope a query-authored
//! label used to land in too. That is the shape the collision actually had, and
//! it is the shape a host hits.
//!
//! # Both directions are exercised
//!
//! Separating scopes is a refusal — the engine now declines to identify two nodes
//! it used to identify — so every case below is paired with the neighbouring
//! case that must still SUCCEED:
//!
//! * two occurrences of one label in ONE query are still one node (within a
//!   single literal, across two literals, and through a nested embedded literal);
//! * a bare `_:b` and the `_:b` of a composite literal in the SAME data document
//!   are still one node;
//! * a composite MINTED from a dataset blank (`cdt:List(?b)`) still reads back as
//!   that very dataset blank, and one minted from `BNODE()` still reads back as
//!   that minted node.

use std::sync::Arc;

use purrdf_core::blank_label::LabelAlphabet;
use purrdf_core::cdt_blank::BlankBinding;
use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest,
    SparqlResult, TermValue,
};
use purrdf_sparql_eval::NativeSparqlEngine;

/// The SEP-0009 datatype namespace — the spec's own fixed string, recognized and
/// never minted — plus the `example.org` fixture namespace this repository's
/// fixture rule requires.
const PFX: &str = "PREFIX cdt: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/> \
                   PREFIX ex:  <http://example.org/> ";

const CDT_LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";
const CDT_MAP: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map";

/// The empty dataset — enough for every query whose composites live in literals.
fn empty() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new().freeze().expect("freeze empty")
}

/// A one-quad dataset `ex:s ex:p <object>` built exactly as a SINGLE parsed
/// document is: blank nodes at [`BlankScope::DEFAULT`], and a composite literal
/// bound through the same [`BlankBinding::Decoded`] every text codec uses.
///
/// `object` is either a bare blank label or a `(lexical, datatype)` composite.
fn document(object: Object<'_>) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let o = match object {
        Object::Blank(label) => b.intern_blank(label, BlankScope::DEFAULT),
        Object::Composite { lexical, datatype } => b
            .intern_literal_bound(
                RdfLiteral {
                    lexical_form: lexical.to_owned(),
                    datatype: Some(datatype.to_owned()),
                    language: None,
                    direction: None,
                },
                BlankBinding::Decoded(LabelAlphabet::BlankNodeLabel),
            )
            .expect("the fixture composite literal is well formed"),
    };
    b.push_quad(s, p, o, None);
    b.freeze().expect("freeze fixture")
}

/// The object of the one-quad fixture built by [`document`].
#[derive(Clone, Copy)]
enum Object<'a> {
    /// A bare `_:label` term.
    Blank(&'a str),
    /// A composite literal.
    Composite {
        /// Its lexical form, written exactly as a document spells it.
        lexical: &'a str,
        /// Its composite datatype IRI.
        datatype: &'a str,
    },
}

/// Evaluate an `ASK` against `dataset` and return its boolean.
fn ask(dataset: &Arc<RdfDataset>, body: &str) -> bool {
    let query = format!("{PFX} ASK {{ {body} }}");
    let result = NativeSparqlEngine::new()
        .query(
            dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .unwrap_or_else(|e| panic!("the query must evaluate: {e}\n{query}"));
    match result {
        SparqlResult::Boolean(b) => b,
        other => panic!("an ASK must answer a boolean, got {other:?}"),
    }
}

/// Evaluate a `SELECT` and return the single cell of its single row.
fn one_cell(dataset: &Arc<RdfDataset>, body: &str) -> Option<TermValue> {
    let query = format!("{PFX} SELECT ?v WHERE {{ {body} }}");
    let result = NativeSparqlEngine::new()
        .query(
            dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .unwrap_or_else(|e| panic!("the query must evaluate: {e}\n{query}"));
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT must answer solutions");
    };
    assert_eq!(rows.len(), 1, "expected exactly one row for: {query}");
    rows.into_iter().next().expect("one row").remove(0)
}

// ── The separation: a query-authored label is not the data's ────────────────

/// `bnodes-turtle-sparql-01`, against a SINGLE-DOCUMENT dataset: the `_:b` of a
/// `cdt:List` literal in the QUERY is not the `_:b` of a `cdt:List` literal in
/// the DATA.
#[test]
fn a_query_authored_list_blank_differs_from_the_datas_list_blank() {
    let ds = document(Object::Composite {
        lexical: "[_:b, 43]",
        datatype: CDT_LIST,
    });
    assert!(
        ask(
            &ds,
            r#"BIND( "[_:b, 42]"^^cdt:List AS ?l1 ) ex:s ex:p ?l2 .
               BIND( cdt:get(?l1,1) AS ?e1 ) BIND( cdt:get(?l2,1) AS ?e2 )
               FILTER( isBLANK(?e1) ) FILTER( isBLANK(?e2) ) FILTER( ?e1 != ?e2 )"#
        ),
        "a query's composite literal writes its own blank-node scope, so its _:b \
         must not be the data's _:b"
    );
}

/// `bnodes-turtle-sparql-02` for `cdt:Map`.
#[test]
fn a_query_authored_map_blank_differs_from_the_datas_map_blank() {
    let ds = document(Object::Composite {
        lexical: "{ '1': _:b, '2': 43 }",
        datatype: CDT_MAP,
    });
    assert!(ask(
        &ds,
        r#"BIND( "{ '1': _:b, '2': 42 }"^^cdt:Map AS ?m1 ) ex:s ex:p ?m2 .
           BIND( cdt:get(?m1,'1') AS ?e1 ) BIND( cdt:get(?m2,'1') AS ?e2 )
           FILTER( isBLANK(?e1) ) FILTER( isBLANK(?e2) ) FILTER( ?e1 != ?e2 )"#
    ));
}

/// `bnodes-turtle-sparql-03`/`-04`: the query's embedded label is not the data's
/// BARE `_:b` either — the label crosses the literal boundary in the data, and it
/// must still not cross the document boundary.
#[test]
fn a_query_authored_blank_differs_from_a_bare_blank_in_the_data() {
    let ds = document(Object::Blank("b"));
    assert!(ask(
        &ds,
        r#"BIND( "[_:b, 42]"^^cdt:List AS ?l ) ex:s ex:p ?bn .
           BIND( cdt:get(?l,1) AS ?e1 )
           FILTER( isBLANK(?e1) ) FILTER( isBLANK(?bn) ) FILTER( ?e1 != ?bn )"#
    ));
    assert!(ask(
        &ds,
        r#"BIND( "{ '1': _:b, '2': 42 }"^^cdt:Map AS ?m ) ex:s ex:p ?bn .
           BIND( cdt:get(?m,'1') AS ?e1 )
           FILTER( isBLANK(?e1) ) FILTER( isBLANK(?bn) ) FILTER( ?e1 != ?bn )"#
    ));
}

/// The separation is by SCOPE, not by label: the query-authored node must differ
/// from the data's even though it is a blank node with the same spelling, and
/// `SAMETERM` — the strictest identity test SPARQL has — must agree with `!=`.
#[test]
fn the_separation_holds_under_sameterm() {
    let ds = document(Object::Blank("b"));
    assert!(!ask(
        &ds,
        r#"BIND( "[_:b]"^^cdt:List AS ?l ) ex:s ex:p ?bn .
           BIND( cdt:get(?l,1) AS ?e ) FILTER( SAMETERM(?e, ?bn) )"#
    ));
}

// ── The neighbouring valid cases, which must still succeed ──────────────────

/// `bnodes-sparql-01`: two occurrences of one label inside ONE query literal are
/// one node. The scope separates the query from the data, never the query from
/// itself.
#[test]
fn one_label_twice_in_one_query_literal_is_one_node() {
    assert!(ask(
        &empty(),
        r#"BIND( "[_:b, 42, _:b]"^^cdt:List AS ?l )
           BIND( cdt:get(?l,1) AS ?e1 ) BIND( cdt:get(?l,3) AS ?e3 )
           FILTER( isBLANK(?e1) ) FILTER( isBLANK(?e3) ) FILTER( ?e1 = ?e3 )"#
    ));
}

/// `bnodes-sparql-15`-shaped: one label across TWO query literals is still one
/// node — they are two literals of the same document.
#[test]
fn one_label_across_two_query_literals_is_one_node() {
    assert!(ask(
        &empty(),
        r#"BIND( "[_:b, 42]"^^cdt:List AS ?l1 ) BIND( "[_:b, 43]"^^cdt:List AS ?l2 )
           BIND( cdt:get(?l1,1) AS ?e1 ) BIND( cdt:get(?l2,1) AS ?e2 )
           FILTER( isBLANK(?e1) ) FILTER( isBLANK(?e2) ) FILTER( SAMETERM(?e1, ?e2) )"#
    ));
}

/// `bnodes-sparql-21`: nesting opens no new scope, so a label inside a
/// composite-typed literal EMBEDDED in a query literal is the same node as the
/// outer occurrence. The binding must therefore reach inside nested lexical
/// forms, not just the top level.
#[test]
fn nesting_inside_a_query_literal_opens_no_new_scope() {
    assert!(ask(
        &empty(),
        r#"BIND( "[_:b, 42, '[_:b]'^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List> ]"^^cdt:List AS ?l )
           BIND( cdt:get(?l,1) AS ?e1 )
           BIND( cdt:get(?l,3) AS ?inner ) BIND( cdt:get(?inner,1) AS ?e3 )
           FILTER( isBLANK(?e1) ) FILTER( isBLANK(?e3) ) FILTER( ?e1 = ?e3 )"#
    ));
}

/// Distinct labels in one query stay distinct: the scope must be applied
/// injectively, not by collapsing every query label onto one node.
#[test]
fn distinct_labels_in_one_query_stay_distinct() {
    assert!(ask(
        &empty(),
        r#"BIND( "[_:b1, _:b2]"^^cdt:List AS ?l )
           BIND( cdt:get(?l,1) AS ?e1 ) BIND( cdt:get(?l,2) AS ?e2 )
           FILTER( isBLANK(?e1) ) FILTER( isBLANK(?e2) ) FILTER( ?e1 != ?e2 )"#
    ));
}

/// `bnodes-turtle-01`, unchanged: within ONE data document a bare `_:b` and the
/// `_:b` of a composite literal are one node. This is what the query-side scope
/// must not disturb — it is the rule the whole `bnodes-turtle-*` group rests on.
#[test]
fn the_datas_own_bare_and_embedded_labels_still_agree() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p1 = b.intern_iri("http://example.org/p1");
    let p2 = b.intern_iri("http://example.org/p2");
    let bare = b.intern_blank("b", BlankScope::DEFAULT);
    let composite = b
        .intern_literal_bound(
            RdfLiteral {
                lexical_form: "[_:b, 42]".to_owned(),
                datatype: Some(CDT_LIST.to_owned()),
                language: None,
                direction: None,
            },
            BlankBinding::Decoded(LabelAlphabet::BlankNodeLabel),
        )
        .expect("well formed");
    b.push_quad(s, p1, composite, None);
    b.push_quad(s, p2, bare, None);
    let ds = b.freeze().expect("freeze");

    assert!(ask(
        &ds,
        r"ex:s ex:p1 ?l . ex:s ex:p2 ?bn . BIND( cdt:get(?l,1) AS ?e )
          FILTER( SAMETERM(?e, ?bn) )"
    ));
}

/// A composite MINTED from a DATASET blank still reads back as that very blank —
/// the `cdt:List(?b)` → `cdt:get` round trip the `bnodes-export-*` cases pin.
///
/// This is the neighbouring-valid case for the interner's new "never promote a
/// query-scoped blank" rule: a blank that came from the DATA is not query-scoped,
/// so it must keep resolving to its dataset term.
#[test]
fn a_composite_minted_from_a_dataset_blank_reads_back_as_that_blank() {
    let ds = document(Object::Blank("b"));
    assert!(ask(
        &ds,
        r"ex:s ex:p ?bn . BIND( cdt:List(?bn) AS ?l ) BIND( cdt:get(?l,1) AS ?e )
          FILTER( SAMETERM(?e, ?bn) )"
    ));
}

/// The same round trip for a blank the query MINTED with `BNODE()` — the exact
/// shape `bnodes-export-rdf-01-construct.rq` builds.
#[test]
fn a_composite_minted_from_bnode_reads_back_as_that_bnode() {
    assert!(ask(
        &empty(),
        r"BIND( BNODE() AS ?b ) BIND( cdt:List(?b) AS ?l ) BIND( cdt:get(?l,1) AS ?e )
          FILTER( SAMETERM(?e, ?b) )"
    ));
}

/// A composite literal carrying no blank node at all is untouched, byte for byte:
/// the binding is guarded by the datatype and rewrites only `BLANK_NODE_LABEL`
/// tokens, so map-entry order, quote style and numeric spellings survive.
#[test]
fn a_blank_free_query_literal_keeps_its_lexical_form() {
    let lexical = one_cell(
        &empty(),
        r#"BIND( "{ '2': 01, '1': 'x' }"^^cdt:Map AS ?v )"#,
    )
    .expect("bound");
    let TermValue::Literal {
        lexical_form,
        datatype,
        ..
    } = lexical
    else {
        panic!("a composite literal");
    };
    assert_eq!(lexical_form, "{ '2': 01, '1': 'x' }");
    assert_eq!(datatype, CDT_MAP);
}
