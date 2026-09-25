// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// The differential fixture for the asynchronous lane: the datasets and queries of the
// synchronous suites (`query.test.mjs`, `governors.test.mjs`, `aggregates.test.mjs`),
// widened into a set that covers every operator family the evaluator has — so that a
// lock, borrow or last-in-first-out guard held across a poll shows up as a wrong answer
// (or a panic) when eight jobs interleave at every poll.
//
// Every query is SERVICE-free: the synchronous lane is the oracle, and it has no
// resolver. Nothing here is RDF 1.1-only; the RDF 1.2 rows are first class.

/** Turtle / TriG documents, by name, with the syntax each is written in. */
export const DATASETS = {
  // `query.test.mjs`: a two-graph TriG asset.
  people: {
    syntax: "trig",
    text: `
@prefix ex: <https://e/> .
ex:a ex:knows ex:b .
ex:a ex:name "Ann" .
ex:b ex:name "Bob" .
graph <https://e/g> { ex:c ex:knows ex:a . }
`,
  },
  // `governors.test.mjs`: the governed lane's two names.
  governed: {
    syntax: "turtle",
    text: `
@prefix ex: <https://example.org/> .
ex:a ex:knows ex:b .
ex:a ex:name "Ann" .
ex:b ex:name "Bob" .
`,
  },
  // `governors.test.mjs`: a class hierarchy.
  rdfs: {
    syntax: "turtle",
    text: `
@prefix ex: <https://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
ex:Cat rdfs:subClassOf ex:Animal .
ex:tom rdf:type ex:Cat .
`,
  },
  // `governors.test.mjs`: a reified statement and an asserted triple term.
  rdf12: {
    syntax: "turtle",
    text: `
@prefix ex: <https://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
ex:a ex:knows ex:b ~ ex:r .
ex:r ex:statedBy ex:ann .
ex:claim rdf:reifies <<( ex:c ex:knows ex:d )>> .
ex:claim2 rdf:reifies <<( ex:e ex:says "hello"@en--ltr )>> .
`,
  },
  // `query.test.mjs`: a graph-scoped reifier and annotation.
  graphStar: {
    syntax: "trig",
    text: `
@prefix ex: <https://e/> .
graph <https://e/g> { ex:s ex:p ex:o ~ex:r {| ex:note "n" |} . }
`,
  },
  // `aggregates.test.mjs`: numeric values and weighted animals.
  values: {
    syntax: "turtle",
    text: `
@prefix ex: <https://example.org/> .
ex:a ex:value 1 .
ex:b ex:value 2 .
ex:c ex:value 3 .
ex:Cat <http://www.w3.org/2000/01/rdf-schema#subClassOf> ex:Animal .
ex:tom a ex:Cat ; ex:weight 1 .
ex:felix a ex:Cat ; ex:weight 2 .
ex:garfield a ex:Cat ; ex:weight 3 .
`,
  },
  // A small social graph: chains for property paths, groups for aggregates, optional
  // attributes for OPTIONAL / MINUS / EXISTS.
  social: {
    syntax: "turtle",
    text: `
@prefix ex: <https://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:p0 ex:knows ex:p1 ; ex:age 31 ; ex:team ex:red ; ex:name "Ada" .
ex:p1 ex:knows ex:p2 ; ex:age 42 ; ex:team ex:red ; ex:name "Bo" ; ex:email "bo@example.org" .
ex:p2 ex:knows ex:p3 ; ex:age 27 ; ex:team ex:blue ; ex:name "Cy" .
ex:p3 ex:knows ex:p4 ; ex:age 55 ; ex:team ex:blue ; ex:name "Di" ; ex:email "di@example.org" .
ex:p4 ex:knows ex:p0 ; ex:age 19 ; ex:team ex:green ; ex:name "Ed" .
ex:p5 ex:age 64 ; ex:team ex:green ; ex:name "Flo"@en .
ex:p1 ex:blocks ex:p4 .
ex:red ex:label "Red" . ex:blue ex:label "Blue" . ex:green ex:label "Green" .
`,
  },
};

const EXO = "PREFIX ex: <https://example.org/> ";
const EXE = "PREFIX ex: <https://e/> ";
const RDF = "PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> ";

/**
 * `{ name, data, query }` — `data` names a `DATASETS` entry. Every name is distinct, and
 * so is every query text.
 */
export const QUERIES = [
  // ── From query.test.mjs ──────────────────────────────────────────────────────
  { name: "select-names-ordered", data: "people", query: `${EXE}SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name` },
  { name: "select-default-graph-only", data: "people", query: `${EXE}SELECT ?o WHERE { ?s ex:knows ?o }` },
  { name: "ask-true", data: "people", query: `${EXE}ASK { ex:a ex:knows ex:b }` },
  { name: "ask-false", data: "people", query: `${EXE}ASK { ex:b ex:knows ex:a }` },
  { name: "select-person-name", data: "people", query: `${EXE}SELECT ?person ?name WHERE { ?person ex:name ?name } ORDER BY ?name` },
  { name: "construct-label", data: "people", query: `${EXE}CONSTRUCT { ?p ex:label ?name } WHERE { ?p ex:name ?name }` },
  { name: "construct-graph-template", data: "people", query: `${EXE}CONSTRUCT { GRAPH ex:out { ?s ex:knows ?o } } WHERE { ?s ex:knows ?o }` },
  { name: "construct-plain", data: "people", query: `${EXE}CONSTRUCT { ?s ex:knows ?o } WHERE { ?s ex:knows ?o }` },
  { name: "describe-graph-star", data: "graphStar", query: "DESCRIBE <https://e/s>" },
  { name: "select-named-graph", data: "people", query: `${EXE}SELECT ?g ?s ?o WHERE { GRAPH ?g { ?s ex:knows ?o } }` },
  { name: "select-union-graphs", data: "people", query: `${EXE}SELECT ?s ?o WHERE { { ?s ex:knows ?o } UNION { GRAPH ?g { ?s ex:knows ?o } } }` },

  // ── From governors.test.mjs ──────────────────────────────────────────────────
  { name: "select-names-governed", data: "governed", query: `${EXO}SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name` },
  { name: "select-rdfs-type", data: "rdfs", query: `${EXO}${RDF}SELECT ?x ?super WHERE { ?x rdf:type ?c . ?c <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?super }` },
  { name: "select-concat", data: "people", query: `${EXE}SELECT (CONCAT(?name, "!") AS ?greeting) WHERE { ?p ex:name ?name }` },
  { name: "count-cross-product", data: "social", query: "SELECT (COUNT(*) AS ?n) WHERE { ?a ?b ?c . ?d ?e ?f }" },
  { name: "rdf12-reifies-select", data: "rdf12", query: `${EXO}${RDF}SELECT ?s ?p ?o WHERE { ex:claim rdf:reifies <<( ?s ?p ?o )>> }` },
  { name: "rdf12-construct-all", data: "rdf12", query: `${EXO}CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }` },

  // ── From aggregates.test.mjs (built-in aggregates; the registry is governed-only) ──
  { name: "aggregate-sum-avg", data: "values", query: `${EXO}SELECT (SUM(?v) AS ?sum) (AVG(?v) AS ?avg) (MIN(?v) AS ?min) (MAX(?v) AS ?max) WHERE { ?s ex:value ?v }` },
  { name: "aggregate-weights-by-class", data: "values", query: `${EXO}SELECT ?c (SUM(?w) AS ?total) WHERE { ?s a ?c ; ex:weight ?w } GROUP BY ?c` },
  { name: "subquery-max-then-join", data: "values", query: `${EXO}SELECT ?s ?v WHERE { ?s ex:value ?v { SELECT (MAX(?x) AS ?v) WHERE { ?t ex:value ?x } } }` },

  // ── Basic graph patterns and joins ───────────────────────────────────────────
  { name: "bgp-two-hop", data: "social", query: `${EXO}SELECT ?a ?c WHERE { ?a ex:knows ?b . ?b ex:knows ?c }` },
  { name: "bgp-star", data: "social", query: `${EXO}SELECT ?p ?n ?a ?t WHERE { ?p ex:name ?n ; ex:age ?a ; ex:team ?t }` },
  { name: "bgp-all-triples", data: "social", query: "SELECT ?s ?p ?o WHERE { ?s ?p ?o }" },

  // ── OPTIONAL, MINUS, EXISTS ──────────────────────────────────────────────────
  { name: "optional-email", data: "social", query: `${EXO}SELECT ?p ?e WHERE { ?p ex:name ?n OPTIONAL { ?p ex:email ?e } }` },
  { name: "optional-nested", data: "social", query: `${EXO}SELECT ?p ?q ?r WHERE { ?p ex:name ?n OPTIONAL { ?p ex:knows ?q OPTIONAL { ?q ex:knows ?r } } }` },
  { name: "optional-filtered", data: "social", query: `${EXO}SELECT ?p ?a WHERE { ?p ex:name ?n OPTIONAL { ?p ex:age ?a FILTER(?a > 40) } }` },
  { name: "minus-emails", data: "social", query: `${EXO}SELECT ?p WHERE { ?p ex:name ?n MINUS { ?p ex:email ?e } }` },
  { name: "filter-exists", data: "social", query: `${EXO}SELECT ?p WHERE { ?p ex:name ?n FILTER EXISTS { ?p ex:knows ?q . ?q ex:email ?e } }` },
  { name: "filter-not-exists", data: "social", query: `${EXO}SELECT ?p WHERE { ?p ex:name ?n FILTER NOT EXISTS { ?x ex:blocks ?p } }` },

  // ── UNION and FILTER ─────────────────────────────────────────────────────────
  { name: "union-teams", data: "social", query: `${EXO}SELECT ?p ?t WHERE { { ?p ex:team ex:red BIND(ex:red AS ?t) } UNION { ?p ex:team ex:green BIND(ex:green AS ?t) } }` },
  { name: "filter-regex-lang", data: "social", query: `${EXO}SELECT ?p ?n WHERE { ?p ex:name ?n FILTER(REGEX(?n, "^[A-D]") || LANG(?n) = "en") }` },
  { name: "filter-in-arith", data: "social", query: `${EXO}SELECT ?p ?a WHERE { ?p ex:age ?a FILTER(?a * 2 IN (62, 84, 110)) }` },

  // ── Subqueries ───────────────────────────────────────────────────────────────
  { name: "subquery-oldest-per-team", data: "social", query: `${EXO}SELECT ?t ?p WHERE { ?p ex:team ?t ; ex:age ?a { SELECT ?t (MAX(?x) AS ?a) WHERE { ?q ex:team ?t ; ex:age ?x } GROUP BY ?t } }` },
  { name: "subquery-limited", data: "social", query: `${EXO}SELECT ?p ?n WHERE { { SELECT ?p WHERE { ?p ex:age ?a } ORDER BY DESC(?a) LIMIT 2 } ?p ex:name ?n }` },

  // ── Aggregates, GROUP BY, HAVING ─────────────────────────────────────────────
  { name: "group-count-having", data: "social", query: `${EXO}SELECT ?t (COUNT(?p) AS ?members) WHERE { ?p ex:team ?t } GROUP BY ?t HAVING (COUNT(?p) >= 2)` },
  { name: "group-concat-sample", data: "social", query: `${EXO}SELECT ?t (GROUP_CONCAT(?n; SEPARATOR="|") AS ?names) (COUNT(DISTINCT ?n) AS ?distinct) WHERE { ?p ex:team ?t ; ex:name ?n } GROUP BY ?t ORDER BY ?t` },
  { name: "aggregate-avg-age-by-team", data: "social", query: `${EXO}SELECT ?t (AVG(?a) AS ?mean) WHERE { ?p ex:team ?t ; ex:age ?a } GROUP BY ?t HAVING (AVG(?a) > 30)` },

  // ── Solution modifiers ───────────────────────────────────────────────────────
  { name: "order-limit-offset", data: "social", query: `${EXO}SELECT ?p ?a WHERE { ?p ex:age ?a } ORDER BY DESC(?a) ?p LIMIT 3 OFFSET 1` },
  { name: "distinct-teams", data: "social", query: `${EXO}SELECT DISTINCT ?t WHERE { ?p ex:team ?t } ORDER BY ?t` },
  { name: "reduced-knows", data: "social", query: `${EXO}SELECT REDUCED ?q WHERE { ?p ex:knows ?q }` },

  // ── Property paths ───────────────────────────────────────────────────────────
  { name: "path-one-or-more", data: "social", query: `${EXO}SELECT ?q WHERE { ex:p0 ex:knows+ ?q }` },
  { name: "path-zero-or-more-sequence", data: "social", query: `${EXO}SELECT ?p ?l WHERE { ?p ex:knows*/ex:team/ex:label ?l }` },
  { name: "path-inverse-alternative", data: "social", query: `${EXO}SELECT ?p ?q WHERE { ?p (^ex:knows|ex:blocks) ?q }` },
  { name: "path-negated", data: "social", query: `${EXO}SELECT ?p ?o WHERE { ?p !(ex:knows|ex:name|ex:age) ?o }` },

  // ── BIND and VALUES ──────────────────────────────────────────────────────────
  { name: "bind-computed", data: "social", query: `${EXO}SELECT ?p ?decade WHERE { ?p ex:age ?a BIND(FLOOR(?a / 10) * 10 AS ?decade) }` },
  { name: "values-inline", data: "social", query: `${EXO}SELECT ?p ?n WHERE { VALUES ?p { ex:p1 ex:p3 ex:nobody } ?p ex:name ?n }` },
  { name: "values-trailing", data: "social", query: `${EXO}SELECT ?p ?t WHERE { ?p ex:team ?t } VALUES ?t { ex:blue ex:green }` },

  // ── CONSTRUCT, ASK, DESCRIBE ─────────────────────────────────────────────────
  { name: "construct-inverse", data: "social", query: `${EXO}CONSTRUCT { ?q ex:knownBy ?p } WHERE { ?p ex:knows ?q }` },
  { name: "construct-where-shorthand", data: "social", query: `${EXO}CONSTRUCT WHERE { ?p ex:email ?e }` },
  { name: "ask-path", data: "social", query: `${EXO}ASK { ex:p0 ex:knows+ ex:p0 }` },
  { name: "describe-person", data: "social", query: `${EXO}DESCRIBE ex:p1` },

  // ── RDF 1.2 triple terms ─────────────────────────────────────────────────────
  { name: "rdf12-reifier-annotation", data: "rdf12", query: `${EXO}${RDF}SELECT ?r ?who WHERE { ?r rdf:reifies <<( ex:a ex:knows ex:b )>> ; ex:statedBy ?who }` },
  { name: "rdf12-triple-functions", data: "rdf12", query: `${RDF}SELECT ?s ?o WHERE { ?r rdf:reifies ?t FILTER(isTRIPLE(?t)) BIND(SUBJECT(?t) AS ?s) BIND(OBJECT(?t) AS ?o) }` },
  { name: "rdf12-directional-literal", data: "rdf12", query: `${RDF}SELECT ?o ?dir WHERE { ?r rdf:reifies <<( ?s ?p ?o )>> FILTER(isLITERAL(?o)) BIND(LANGDIR(?o) AS ?dir) }` },
  { name: "rdf12-construct-triple-term", data: "rdf12", query: `${EXO}${RDF}CONSTRUCT { ?r ex:about ?t } WHERE { ?r rdf:reifies ?t }` },
];
