// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The property-function seam, end to end, from the vantage a host has.
//!
//! One query text is carried through every stage the seam touches — parse under a
//! configured namespace, the algebra node the parse produced, evaluation against a
//! host-injected relation, the answers — and then through the serializer and back,
//! byte-exactly. Nothing here reaches into the crate: a seam whose stages only line up
//! from inside is a seam a host cannot use.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_algebra::{
    GraphPattern, Query, SparqlParser, TermPattern, Variable, pattern_to_select_query,
};
use purrdf_sparql_eval::{
    MemoryRelation, NativeSparqlEngine, ParserOptions, PropertyFunctionRegistry,
};

/// The namespace this host configured. PurRDF mints none: without this line in the
/// parser options, `rel:memberOf` is an ordinary predicate and the query below reads
/// the graph instead of the relation.
const REL_NS: &str = "https://example.org/rel/";

/// The data namespace of the fixture terms.
const EX: &str = "https://example.org/d/";

/// The one query text every stage below is given.
const QUERY: &str = "PREFIX rel: <https://example.org/rel/>\n\
                     SELECT ?person ?team WHERE { ?person rel:memberOf ?team }\n";

fn options() -> ParserOptions {
    ParserOptions {
        extension_fn_namespaces: Vec::new(),
        property_fn_namespaces: vec![REL_NS.to_owned()],
        property_fn_iris: Vec::new(),
    }
}

/// The host's relation: three (person, team) pairs, held in host memory and reachable
/// from no graph.
fn relations() -> PropertyFunctionRegistry {
    let iri = |local: &str| TermValue::iri(format!("{EX}{local}"));
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        format!("{REL_NS}memberOf"),
        Arc::new(
            MemoryRelation::new(
                1,
                1,
                vec![
                    vec![iri("ada"), iri("alpha")],
                    vec![iri("brian"), iri("alpha")],
                    vec![iri("chen"), iri("beta")],
                ],
            )
            .expect("every row is two values wide"),
        ),
    );
    registry
}

/// A dataset holding one unrelated triple: the answers below come from the relation,
/// and this is what makes that observable rather than merely stated.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(&format!("{EX}unrelated"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let o = builder.intern_iri(&format!("{EX}o"));
    builder.push_quad(s, p, o, None);
    builder.freeze().expect("freeze fixture")
}

fn variable(name: &str) -> TermPattern {
    TermPattern::Variable(Variable::new(name.to_owned()))
}

/// The call node in the parsed query, with the shape the parser is contracted to
/// produce: the call joins through a `Lateral` onto whatever was written before it,
/// which for a lone call is the empty basic graph pattern (the identity table).
fn call_of(query: &Query) -> &purrdf_sparql_algebra::PropertyFunctionCall {
    let Query::Select { pattern, .. } = query else {
        panic!("the acceptance query is a SELECT");
    };
    let GraphPattern::Project { inner, .. } = pattern else {
        panic!("a SELECT's algebra root is a Project, got {pattern:?}");
    };
    let GraphPattern::Lateral { left, right } = &**inner else {
        panic!("a call joins through a Lateral, got {inner:?}");
    };
    assert!(
        matches!(&**left, GraphPattern::Bgp { patterns } if patterns.is_empty()),
        "a call written first is driven by the identity table, got {left:?}"
    );
    let GraphPattern::PropertyFunction(call) = &**right else {
        panic!("the Lateral's right operand is the call, got {right:?}");
    };
    call
}

/// Text → parse → algebra → evaluation → answers, in one pass.
#[test]
fn a_configured_predicate_parses_to_a_call_and_answers_from_the_injected_relation() {
    // 1. Parse, with the namespace configured.
    let query = SparqlParser::new()
        .parse_query_with(QUERY, &options())
        .expect("the query parses under the configured namespace");

    // 2. The algebra carries the call, with the predicate IRI byte-exact and the two
    //    argument vectors in written order.
    let call = call_of(&query);
    assert_eq!(call.iri, format!("{REL_NS}memberOf"));
    assert_eq!(call.subject_args, vec![variable("person")]);
    assert_eq!(call.object_args, vec![variable("team")]);

    // 3. Evaluate against the injected relation. The engine derives the parse-time
    //    namespace from the registry itself, so a host that registers a relation does
    //    not also have to configure the parser for it.
    let result = NativeSparqlEngine::new()
        .query_with_property_functions(
            &dataset(),
            SparqlRequest {
                query: QUERY,
                base_iri: None,
                substitutions: &[],
            },
            &relations(),
        )
        .expect("the call resolves and evaluates");

    // 4. The answers are the relation's rows, in the relation's emission order.
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("a SELECT returns solutions");
    };
    assert_eq!(variables, vec!["person".to_owned(), "team".to_owned()]);
    let rendered: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| {
                    cell.as_ref()
                        .and_then(TermValue::as_iri)
                        .expect("every cell is a bound IRI")
                        .to_owned()
                })
                .collect()
        })
        .collect();
    assert_eq!(
        rendered,
        vec![
            vec![format!("{EX}ada"), format!("{EX}alpha")],
            vec![format!("{EX}brian"), format!("{EX}alpha")],
            vec![format!("{EX}chen"), format!("{EX}beta")],
        ]
    );
}

/// The same query, serialized out of the algebra and read back in: the round trip is
/// exact in both directions — the algebra survives it, and so do the bytes.
#[test]
fn the_call_serializes_and_re_parses_byte_exactly() {
    let query = SparqlParser::new()
        .parse_query_with(QUERY, &options())
        .expect("parse");
    let Query::Select { pattern, .. } = &query else {
        panic!("the acceptance query is a SELECT");
    };
    let GraphPattern::Project { inner, .. } = pattern else {
        panic!("a SELECT's algebra root is a Project");
    };

    let serialized = pattern_to_select_query(inner);
    assert!(
        serialized.contains(&format!("<{REL_NS}memberOf>")),
        "the emitted text carries the predicate IRI as written, never a fabricated \
         prefix: {serialized}"
    );

    let reparsed = SparqlParser::new()
        .parse_query_with(&serialized, &options())
        .expect("the serialized text parses under the same options");
    let Query::Select {
        pattern: reparsed_pattern,
        ..
    } = &reparsed
    else {
        panic!("the serialized text is a SELECT");
    };
    let GraphPattern::Project {
        inner: reparsed_inner,
        ..
    } = reparsed_pattern
    else {
        panic!("a SELECT's algebra root is a Project");
    };
    assert_eq!(
        &**reparsed_inner, &**inner,
        "the algebra survives the round trip"
    );
    assert_eq!(
        pattern_to_select_query(reparsed_inner),
        serialized,
        "and so do the bytes"
    );
    assert_eq!(
        call_of(&reparsed).iri,
        call_of(&query).iri,
        "the predicate IRI is byte-exact on the way back"
    );
}

/// Without the namespace, the very same text is an ordinary triple pattern reading the
/// graph — the seam is configuration, and there is no default that turns it on.
#[test]
fn the_same_text_without_the_namespace_is_an_ordinary_triple_pattern() {
    let query = SparqlParser::new()
        .parse_query(QUERY)
        .expect("the text is valid SPARQL either way");
    let Query::Select { pattern, .. } = &query else {
        panic!("the acceptance query is a SELECT");
    };
    let GraphPattern::Project { inner, .. } = pattern else {
        panic!("a SELECT's algebra root is a Project");
    };
    let GraphPattern::Bgp { patterns } = &**inner else {
        panic!("with no namespace configured the body is a basic graph pattern, got {inner:?}");
    };
    assert_eq!(patterns.len(), 1);

    // And it answers from the graph, which holds no such triple.
    let result = NativeSparqlEngine::new()
        .query(
            &dataset(),
            SparqlRequest {
                query: QUERY,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("an unconfigured engine evaluates it as data");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT returns solutions");
    };
    assert!(
        rows.is_empty(),
        "the dataset holds no rel:memberOf triple: {rows:?}"
    );
}

/// THE data-predicate-hijack regression (GAP-3): registering a relation must not
/// turn a merely prefix-sharing, unregistered, LONGER predicate into a
/// hard-erroring property-function call.
///
/// `NativeSparqlEngine::prepare_for` derives parse-time recognition from the
/// registry (see the doc comment there): it used to push each registered
/// relation's exact IRI into the parser's PREFIX namespace set, so registering
/// `{REL_NS}a` made the ordinary, unrelated data predicate `{REL_NS}ab` parse as
/// an (unregistered) property-function call and hard-error — a previously
/// working query breaking with a diagnostic that names the wrong cause. A
/// registry's keys are exact IRIs, not namespaces, and exact match is the only
/// rule that respects that.
#[test]
fn registering_a_relation_does_not_hijack_a_longer_sibling_data_predicate() {
    let short_iri = format!("{REL_NS}a");
    let long_predicate = format!("{REL_NS}ab");

    // Register a relation under the SHORT IRI only.
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        short_iri,
        Arc::new(
            MemoryRelation::new(
                1,
                1,
                vec![vec![
                    TermValue::iri(format!("{EX}from_relation_x")),
                    TermValue::iri(format!("{EX}from_relation_y")),
                ]],
            )
            .expect("one row, two values wide"),
        ),
    );

    // The dataset holds an ordinary triple under the LONGER, unregistered IRI —
    // it merely shares the short IRI's characters as a prefix.
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(&format!("{EX}subject"));
    let p = builder.intern_iri(&long_predicate);
    let o = builder.intern_iri(&format!("{EX}object"));
    builder.push_quad(s, p, o, None);
    let dataset = builder.freeze().expect("freeze fixture");

    let query = format!("SELECT ?s ?o WHERE {{ ?s <{long_predicate}> ?o }}");
    let result = NativeSparqlEngine::new()
        .query_with_property_functions(
            &dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            &registry,
        )
        .expect(
            "an unregistered, merely-prefix-sharing predicate must parse and evaluate as an \
             ordinary data triple, never a hard-erroring call",
        );

    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT returns solutions");
    };
    assert_eq!(
        rows.len(),
        1,
        "the triple under the longer predicate is read from the graph as an ordinary BGP \
         triple, not routed through the relation: {rows:?}"
    );
}
