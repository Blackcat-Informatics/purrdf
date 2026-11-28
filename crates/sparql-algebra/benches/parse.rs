// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Parse throughput over a fixed set of thirty realistic queries.
//!
//! The set is embedded rather than read from the W3C conformance corpus, so the
//! bench needs no file access and no dependency outside this crate, and it stays
//! the same text across revisions. It spans what a query parser meets in practice:
//! the four query forms; star, chain and snowflake joins; `OPTIONAL`, `UNION`,
//! `MINUS` and `GRAPH`; `FILTER` with value comparisons, `REGEX`, `IN`, string and
//! date built-ins; `BIND` and `VALUES`; `EXISTS`/`NOT EXISTS`; property paths;
//! `GROUP BY` with aggregates and `HAVING`; sub-`SELECT`s; solution modifiers; RDF
//! 1.2 triple terms and annotations; and `LATERAL`. All IRIs are `example.org`
//! fixtures, apart from the standard `rdf:`, `rdfs:` and `xsd:` namespaces.
//!
//! `parse/all` parses the whole set once per sample, with its total length as the
//! byte throughput; `parse/<query>` parses one query, so a regression in one
//! production is attributable. The parsed tree is dropped inside each sample.
//!
//! Report-only, `cargo bench -p purrdf-sparql-algebra --bench parse` (the
//! `make bench` lane) — excluded from `make check`. No timing is asserted.

use purrdf_sparql_algebra::SparqlParser;
use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main, black_box};

/// The shared prologue, prefixed to every query below.
const PROLOGUE: &str = "\
PREFIX ex: <https://example.org/>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
";

/// `(bench id, query body)`.
const QUERIES: &[(&str, &str)] = &[
    ("select_one_pattern", "SELECT ?s ?o WHERE { ?s ex:p ?o }"),
    (
        "select_star_join",
        "SELECT ?p ?name ?age ?city WHERE { ?p a ex:Person ; ex:name ?name ; ex:age ?age ; ex:city ?city . }",
    ),
    (
        "select_chain_join",
        "SELECT ?a ?d WHERE { ?a ex:knows ?b . ?b ex:knows ?c . ?c ex:knows ?d . ?d ex:worksFor ex:acme . }",
    ),
    (
        "select_snowflake",
        "SELECT ?p ?dn ?cn WHERE { ?p ex:dept ?d ; ex:city ?c . ?d rdfs:label ?dn ; ex:head ?h . ?c rdfs:label ?cn ; ex:country ?k . ?k rdfs:label \"Canada\"@en . }",
    ),
    (
        "filter_numeric",
        "SELECT ?p WHERE { ?p ex:age ?a ; ex:salary ?s . FILTER(?a >= 18 && ?a < 65 && ?s > 50000.0) }",
    ),
    (
        "filter_regex",
        "SELECT ?p ?n WHERE { ?p ex:name ?n . FILTER(REGEX(?n, \"^[A-M][a-z]+$\", \"i\")) }",
    ),
    (
        "filter_in",
        "SELECT ?p WHERE { ?p ex:status ?st . FILTER(?st IN (ex:active, ex:pending, ex:review) && ?st NOT IN (ex:banned)) }",
    ),
    (
        "filter_strings",
        "SELECT ?p WHERE { ?p ex:email ?e . FILTER(STRENDS(LCASE(STR(?e)), \"@example.org\") && CONTAINS(?e, \".\") && STRLEN(?e) > 5) }",
    ),
    (
        "filter_dates",
        "SELECT ?e WHERE { ?e ex:start ?t . FILTER(?t >= \"2024-01-01T00:00:00Z\"^^xsd:dateTime && YEAR(?t) = 2024 && MONTH(?t) IN (1, 2, 3)) }",
    ),
    (
        "optional_chain",
        "SELECT ?p ?e ?ph ?w WHERE { ?p a ex:Person . OPTIONAL { ?p ex:email ?e } OPTIONAL { ?p ex:phone ?ph } OPTIONAL { ?p ex:homepage ?w FILTER(isIRI(?w)) } }",
    ),
    (
        "union_branches",
        "SELECT ?x ?label WHERE { { ?x rdfs:label ?label } UNION { ?x ex:name ?label } UNION { ?x ex:title ?label FILTER(LANG(?label) = \"en\") } }",
    ),
    (
        "minus_anti_join",
        "SELECT ?p WHERE { ?p a ex:Person . MINUS { ?p ex:status ex:inactive } MINUS { ?p ex:deleted true } }",
    ),
    (
        "graph_named",
        "SELECT ?g ?s ?o WHERE { GRAPH ?g { ?s ex:p ?o } GRAPH ex:meta { ?g ex:source ?src } }",
    ),
    (
        "bind_values",
        "SELECT ?p ?full ?band WHERE { VALUES ?kind { ex:Person ex:Agent } ?p a ?kind ; ex:first ?f ; ex:last ?l ; ex:age ?a . BIND(CONCAT(?f, \" \", ?l) AS ?full) BIND(IF(?a < 30, \"young\", IF(?a < 60, \"middle\", \"senior\")) AS ?band) }",
    ),
    (
        "exists_not_exists",
        "SELECT ?p WHERE { ?p a ex:Person . FILTER EXISTS { ?p ex:knows ?q } FILTER NOT EXISTS { ?p ex:blocked ?r } }",
    ),
    (
        "path_sequence_alternative",
        "SELECT ?a ?c WHERE { ?a ex:parent/ex:parent ?c . ?a (ex:knows|ex:worksWith) ?b . ?b ^ex:manages ?m . }",
    ),
    (
        "path_closure",
        "SELECT ?x ?super WHERE { ?x rdf:type/rdfs:subClassOf* ?super . ?super rdfs:subClassOf+ ex:Thing . ?x !(rdf:type|ex:hidden) ?y . }",
    ),
    (
        "group_aggregates",
        "SELECT ?d (COUNT(DISTINCT ?p) AS ?n) (AVG(?s) AS ?avg) (MIN(?s) AS ?lo) (MAX(?s) AS ?hi) (SUM(?s) AS ?total) WHERE { ?p ex:dept ?d ; ex:salary ?s } GROUP BY ?d",
    ),
    (
        "group_having",
        "SELECT ?c (SAMPLE(?n) AS ?one) (GROUP_CONCAT(?n; separator=\", \") AS ?names) WHERE { ?p ex:city ?c ; ex:name ?n } GROUP BY ?c HAVING (COUNT(?p) > 10)",
    ),
    (
        "subselect",
        "SELECT ?p ?top WHERE { ?p ex:dept ?d . { SELECT ?d (MAX(?s) AS ?top) WHERE { ?q ex:dept ?d ; ex:salary ?s } GROUP BY ?d } }",
    ),
    (
        "modifiers",
        "SELECT DISTINCT ?p ?n WHERE { ?p ex:name ?n ; ex:age ?a } ORDER BY DESC(?a) ASC(?n) LIMIT 50 OFFSET 100",
    ),
    ("ask", "ASK { ?p ex:email \"p1@example.org\" }"),
    (
        "construct_template",
        "CONSTRUCT { ?p ex:label ?n ; ex:inDept ?d . ?d ex:member ?p } WHERE { ?p ex:name ?n ; ex:dept ?d }",
    ),
    (
        "construct_where",
        "CONSTRUCT WHERE { ?p ex:name ?n ; ex:age ?a }",
    ),
    (
        "describe",
        "DESCRIBE ?p ex:alice WHERE { ?p ex:knows ex:alice }",
    ),
    (
        "triple_terms",
        "SELECT ?s ?src WHERE { ?r rdf:reifies <<( ?s ex:claims ?o )>> ; ex:source ?src . FILTER(isTRIPLE(<<( ?s ex:claims ?o )>>)) }",
    ),
    (
        "annotations",
        "SELECT ?s ?o ?c WHERE { ?s ex:rated ?o {| ex:confidence ?c ; ex:by ?who |} . FILTER(?c > 0.8) }",
    ),
    (
        "lateral",
        "SELECT ?p ?f WHERE { ?p a ex:Person . LATERAL { SELECT ?f WHERE { ?p ex:knows ?f } ORDER BY ?f LIMIT 3 } }",
    ),
    (
        "builtins_mix",
        "SELECT ?p (STRUUID() AS ?id) ?h WHERE { ?p ex:name ?n . BIND(SHA256(STR(?n)) AS ?h) BIND(COALESCE(?missing, UCASE(SUBSTR(?n, 1, 3))) AS ?tag) FILTER(BOUND(?h) && sameTerm(DATATYPE(?n), xsd:string)) }",
    ),
    (
        "service_federated",
        "SELECT ?p ?remote WHERE { ?p a ex:Person . SERVICE SILENT <https://example.org/sparql> { ?p ex:remoteName ?remote } }",
    ),
];

fn bench_parse(c: &mut Bench) {
    let parser = SparqlParser::new();
    let texts: Vec<(&str, String)> = QUERIES
        .iter()
        .map(|&(id, body)| (id, format!("{PROLOGUE}{body}")))
        .collect();
    // Every query parses before any is timed: a refusal would benchmark the error path.
    for (id, text) in &texts {
        if let Err(error) = parser.parse_query(text) {
            panic!("{id} parses: {error}");
        }
    }
    let total: usize = texts.iter().map(|(_, text)| text.len()).sum();

    let mut group = c.benchmark_group("parse");
    group.throughput(Throughput::Bytes(total as u64));
    group.bench_function("all", |b| {
        b.iter(|| {
            for (_, text) in &texts {
                black_box(parser.parse_query(black_box(text)).expect("parses"));
            }
        });
    });
    for (id, text) in &texts {
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_function(*id, |b| {
            b.iter(|| black_box(parser.parse_query(black_box(text)).expect("parses")));
        });
    }
    group.finish();
}

bench_group!(benches, bench_parse);
bench_main!(benches);
