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
test("shaclValidateToSarif honours conformanceDisallows", () => {
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

// Every sh:message reaches the SARIF result, each with its language tag.
test("shaclValidateToSarif carries every message with its language", () => {
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
