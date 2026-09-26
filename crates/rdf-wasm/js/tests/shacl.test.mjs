// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution conformance for the SHACL surface reached through the PUBLIC
// package root (`../index.mjs`) — `shaclValidateToSarif`,
// `shaclValidateChangesToSarif` and `shaclEntail`, exactly as the docs playground's
// SHACL pane calls them in a browser. Proves the SHACL engine is reachable from the
// shipped package (not only a deep `./pkg/` import).

import { test } from "node:test";
import assert from "node:assert/strict";

import {
  ready,
  shaclEntail,
  shaclValidateChangesToSarif,
  shaclValidateToSarif,
} from "../index.mjs";

await ready();

// Shapes as Turtle, data as N-Triples — the exact input contract of the two functions.
const SHAPES = `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:PersonShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .
`;

const DATA = `<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .
<http://example.org/alice> <http://example.org/age> "nope" .
`;

test("shaclValidateToSarif emits SARIF 2.1.0 with a violation", () => {
  const sarif = JSON.parse(shaclValidateToSarif(SHAPES, DATA));
  assert.equal(sarif.version, "2.1.0");
  const results = sarif.runs.flatMap((r) => r.results ?? []);
  assert.ok(results.length >= 1, "the ill-typed age must produce at least one result");
  assert.ok(
    results.some((r) => r.level === "error"),
    "a datatype violation is an error-level SARIF result",
  );
});

test("shaclValidateToSarif rejects malformed shapes (never a silent pass)", () => {
  assert.throws(() => shaclValidateToSarif("@@@ not turtle", DATA));
});

// The conformance-disallow set: a Warning-graded violation does not conform under the
// default set and conforms under sh:Violation alone; a non-IRI level throws.
test("wasm_shacl_conformance_disallows: shaclValidateToSarif honours conformanceDisallows", () => {
  const warning = SHAPES.replace(
    "sh:path ex:age ;",
    "sh:path ex:age ; sh:severity sh:Warning ;",
  );
  const run = (disallows) =>
    JSON.parse(shaclValidateToSarif(warning, DATA, undefined, disallows)).runs[0].properties;
  assert.equal(run(undefined).shaclConforms, false);
  const relaxed = run(["http://www.w3.org/ns/shacl#Violation"]);
  assert.equal(relaxed.shaclConforms, true);
  assert.deepEqual(relaxed.shaclConformanceDisallows, [
    "http://www.w3.org/ns/shacl#Violation",
  ]);
  assert.throws(() => shaclValidateToSarif(warning, DATA, undefined, ["Violation"]));
});

// The W3C SHACL 1.2 vocabulary's declaration of the built-in sh:SPARQLExprExpression,
// verbatim: a sh:NamedParameterExpressionFunction with the two sh:Parameters -prefixes
// and -sparqlExpr and no sh:bodyExpression.
const SPARQL_EXPR_DECLARATION = `@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sh:SPARQLExprExpression a sh:NamedParameterExpressionFunction ;
  rdfs:label "SPARQL expr expression"@en ;
  rdfs:comment "The class of node expressions based on SPARQL expressions (sh:sparqlExpr)."@en ;
  rdfs:isDefinedBy sh: ;
  rdfs:subClassOf sh:NamedParameterExpression,
  sh:SPARQLExecutable ;
  sh:parameter sh:SPARQLExprExpression-prefixes,
  sh:SPARQLExprExpression-sparqlExpr .

sh:SPARQLExprExpression-prefixes a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:description "The prefixes that shall be applied before parsing the SPARQL query that gets derived from the sh:sparqlExpr expression. The object should define those prefixes using sh:declare."@en ;
  sh:name "prefixes"@en ;
  sh:nodeKind sh:BlankNodeOrIRI ;
  sh:path sh:prefixes .

sh:SPARQLExprExpression-sparqlExpr a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:datatype xsd:string ;
  sh:description "The SPARQL expression that is executed during evaluation of this node expression."@en ;
  sh:keyParameter true ;
  sh:name "SPARQL expr"@en ;
  sh:path sh:sparqlExpr .
`;

// A Warning-graded property shape whose one value node is computed by
// sh:sparqlExpr "ex:active" through sh:prefixes; sh:in ( ex:retired ) refuses it.
const SPARQL_EXPR_SHAPES = `
@prefix ex: <http://example.org/> .
ex:Prefixes sh:declare [ sh:prefix "ex" ; sh:namespace "http://example.org/"^^xsd:anyURI ] .
ex:PersonShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [
    sh:path ex:status ;
    sh:values [ sh:sparqlExpr "ex:active" ; sh:prefixes ex:Prefixes ] ;
    sh:in ( ex:retired ) ;
    sh:severity sh:Warning
  ] .
`;

// On wasm, a shapes graph merged with the SHACL 1.2 vocabulary's own
// sh:SPARQLExprExpression declaration loads, and the sh:sparqlExpr +
// sh:prefixes expression is evaluated natively — the result's value is the computed
// <http://example.org/active>, which only the prefix-expanded expression yields. The
// Warning result does not conform by default and conforms under sh:Violation alone.
test("wasm_shacl_sparql_expr_declaration: shaclValidateToSarif evaluates sh:sparqlExpr beside its vocabulary declaration", () => {
  const shapes = SPARQL_EXPR_DECLARATION + SPARQL_EXPR_SHAPES;
  const run = (disallows) => JSON.parse(shaclValidateToSarif(shapes, DATA, undefined, disallows)).runs[0];
  const byDefault = run(undefined);
  assert.equal(byDefault.properties.shaclConforms, false);
  assert.equal(byDefault.results.length, 1);
  assert.ok(
    byDefault.results[0].message.text.startsWith("Value <http://example.org/active> "),
    byDefault.results[0].message.text,
  );
  assert.equal(run(["http://www.w3.org/ns/shacl#Violation"]).properties.shaclConforms, true);
});

// Every sh:message reaches the SARIF result, each with its language tag.
test("wasm_shacl_message_languages: shaclValidateToSarif carries every message with its language", () => {
  const tagged = SHAPES.replace(
    "sh:datatype xsd:integer ]",
    'sh:datatype xsd:integer ; sh:message "Too many"@en , "Zu viele"@de ]',
  );
  const result = JSON.parse(shaclValidateToSarif(tagged, DATA)).runs[0].results[0];
  assert.equal(result.message.text, "Too many");
  assert.deepEqual(result.properties.shaclMessages, [
    { text: "Too many", language: "en" },
    { text: "Zu viele", language: "de" },
  ]);
});

// A `sh:rule` shapes graph that types every ex:Person as ex:adult ex:yes.
const RULE_SHAPES = `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:PersonRule a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:rule [ a sh:TripleRule ;
    sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .
`;

const RULE_DATA = `<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .
`;

test("shaclEntail materializes the inferred triple and keeps the base fact", () => {
  const nt = shaclEntail(RULE_SHAPES, RULE_DATA);
  assert.match(
    nt,
    /<http:\/\/example\.org\/alice> <http:\/\/example\.org\/adult> <http:\/\/example\.org\/yes> \./,
  );
  assert.match(
    nt,
    /<http:\/\/example\.org\/alice> <http:\/\/www\.w3\.org\/1999\/02\/22-rdf-syntax-ns#type> <http:\/\/example\.org\/Person> \./,
  );
});

// A conforming base, so every violation below is the CHANGE's doing.
const CHANGE_BASE = `<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .
`;

// The row that breaks it.
const BAD_AGE = `<http://example.org/alice> <http://example.org/age> "nope" .
`;

test("shaclValidateChangesToSarif validates the change and reports its scope", () => {
  const outcome = shaclValidateChangesToSarif(SHAPES, CHANGE_BASE, BAD_AGE);
  try {
    assert.equal(outcome.bounded, true);
    assert.equal(outcome.focusNodes, 1);
    assert.equal(outcome.reason, undefined, "a bounded scope names no reason");
    // The change path is a cheaper route to ONE answer: the log is the one a full
    // validation of the merged graph produces.
    assert.equal(outcome.sarif, shaclValidateToSarif(SHAPES, CHANGE_BASE + BAD_AGE));
  } finally {
    outcome.free();
  }
});

test("shaclValidateChangesToSarif honours the retract half", () => {
  const outcome = shaclValidateChangesToSarif(
    SHAPES,
    CHANGE_BASE + BAD_AGE,
    undefined,
    BAD_AGE,
  );
  try {
    assert.equal(outcome.bounded, true);
    const sarif = JSON.parse(outcome.sarif);
    const results = sarif.runs.flatMap((r) => r.results ?? []);
    assert.equal(results.length, 0, "removing the ill-typed row restores conformance");
  } finally {
    outcome.free();
  }
});

// A shapes graph whose constraint reads through SPARQL query text: no bounded
// footprint exists for it, so the change path must fall back to a FULL validation
// and say why. The fallback is not optional — a short expansion and a clean bill of
// health are indistinguishable in a report.
const SPARQL_SHAPES = `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:PersonShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:sparql [ a sh:SPARQLConstraint ;
    sh:message "every person needs a name" ;
    sh:select """SELECT $this WHERE { FILTER NOT EXISTS { $this <http://example.org/name> ?n } }""" ] .
`;

test("shaclValidateChangesToSarif reports an unbounded footprint rather than guessing", () => {
  const outcome = shaclValidateChangesToSarif(SPARQL_SHAPES, CHANGE_BASE, BAD_AGE);
  try {
    assert.equal(outcome.bounded, false);
    assert.equal(outcome.focusNodes, undefined, "a fallback covers no COUNT");
    assert.ok(typeof outcome.reason === "string" && outcome.reason.length > 0);
    assert.equal(outcome.sarif, shaclValidateToSarif(SPARQL_SHAPES, CHANGE_BASE + BAD_AGE));
  } finally {
    outcome.free();
  }
});

test("shaclValidateChangesToSarif rejects a malformed change document", () => {
  assert.throws(() => shaclValidateChangesToSarif(SHAPES, CHANGE_BASE, "@@@ not n-triples"));
  // The neighbouring VALID case still succeeds — a refusal is a claim too.
  const outcome = shaclValidateChangesToSarif(SHAPES, CHANGE_BASE, BAD_AGE);
  outcome.free();
});
