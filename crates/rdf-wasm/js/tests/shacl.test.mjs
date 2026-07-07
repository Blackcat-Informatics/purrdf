// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Node real-execution conformance for the SHACL surface reached through the PUBLIC
// package root (`../index.mjs`) — `shaclValidateToSarif` / `shaclEntail`, exactly as
// the docs playground's SHACL pane calls them in a browser. Proves the SHACL engine is
// reachable from the shipped package (not only a deep `./pkg/` import).

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, shaclValidateToSarif, shaclEntail } from "../index.mjs";

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
