// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Node real-execution conformance for the entailment-REGIME surface reached through
// the PUBLIC package root (`../index.mjs`) — `entailMaterialize` / `entailRules` /
// `entailImplementedRules` / `entailCheckGoldenVectors`. Not to be confused with
// `shaclEntail` (tests/shacl.test.mjs), which is SHACL-AF `sh:rule` entailment over a
// shapes graph; these close a document under a regime's own specification rule table.
//
// This file is where the tri-host claim is actually EXECUTED on wasm32. The repo has
// no `wasm-bindgen-test` harness (the crate carries no such dev-dependency, and adding
// one is out of scope), so the wasm leg of the assertion runs here instead: Node loads
// the real wasm-bindgen artifact `make wasm-pkg` produced and calls the SAME
// `check_regime_golden_vectors()` that the `purrdf-validate` and `purrdf-capi` Rust
// tests call, over the SAME committed artifact. wasm32 has a different pointer width,
// a different `usize`, different float behaviour and different map iteration; if any
// of those leaked into the chase, the canonical serializer or the report renderer,
// this is where it shows up.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import {
  ready,
  entailCheckGoldenVectors,
  entailImplementedRules,
  entailMaterialize,
  entailRules,
} from "../index.mjs";

await ready();

// `A ⊑ B ⊑ C`, and one typed instance — enough for rdfs9 to re-type it twice.
const SCHEMA = `<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .
<http://example.org/B> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .
`;

test("entailCheckGoldenVectors runs the committed tri-host artifact ON WASM", () => {
  // Throws with the case name and a diff of the two strings on the first byte that
  // differs from what the native build produced.
  entailCheckGoldenVectors();
});

// The same artifact, parsed here and driven case by case through the JS boundary — so
// the comparison covers the wasm→JS string marshalling too, not only the byte equality
// the checker establishes inside the module.
const VECTORS = fileURLToPath(
  new URL("../../../validate/tests/fixtures/regime-boundary.vectors", import.meta.url),
);

/** Parse the line-oriented `@case/@regime/@input/@closure/@report/@end` artifact. */
function parseVectors(text) {
  const cases = [];
  let current = {};
  let open = null;
  for (const line of text.split("\n").slice(0, -1)) {
    if (!line.startsWith("@")) {
      if (open !== null) current[open] += `${line}\n`;
      continue;
    }
    const [keyword, ...rest] = line.slice(1).split(" ");
    open = null;
    if (keyword === "case") current.name = rest.join(" ").trim();
    else if (keyword === "regime") current.regime = rest.join(" ").trim();
    else if (keyword === "input" || keyword === "closure" || keyword === "report") {
      current[keyword] = "";
      open = keyword;
    } else if (keyword === "end") {
      cases.push(current);
      current = {};
    }
  }
  return cases;
}

test("every golden case is byte-identical across the wasm/JS boundary", async () => {
  const cases = parseVectors(await readFile(VECTORS, "utf8"));
  assert.ok(cases.length > 0, "the artifact must hold cases");
  for (const vector of cases) {
    const closed = entailMaterialize(vector.input, vector.regime);
    assert.equal(closed.nquads, vector.closure, `${vector.name}: closure`);
    assert.equal(closed.report, vector.report, `${vector.name}: report`);
  }
});

test("entailMaterialize closes under rdfs and always returns a report", () => {
  const closed = entailMaterialize(SCHEMA, "rdfs");
  assert.match(
    closed.nquads,
    /<http:\/\/example\.org\/x> <http:\/\/www\.w3\.org\/1999\/02\/22-rdf-syntax-ns#type> <http:\/\/example\.org\/C> \./,
  );
  assert.ok(closed.report.startsWith("purrdf-reasoning-report 1\n"));
  assert.ok(closed.report.includes("\nregime rdfs\n"));
  // The report names the gap rather than claiming completeness it does not have.
  assert.ok(closed.report.includes("\ncompleteness sound-incomplete 4\n"));
  assert.ok(closed.report.endsWith("overclaims false\n"));
});

test("entailMaterialize under simple is the identity closure", () => {
  const closed = entailMaterialize(SCHEMA, "simple");
  assert.ok(
    !closed.nquads.includes(
      "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .",
    ),
  );
  assert.ok(closed.report.includes("\ncompleteness exact\n"));
});

test("entailMaterialize is byte-stable across repeated calls", () => {
  const first = entailMaterialize(SCHEMA, "owl-rl");
  for (let i = 0; i < 5; i += 1) {
    const again = entailMaterialize(SCHEMA, "owl-rl");
    assert.equal(again.nquads, first.nquads);
    assert.equal(again.report, first.report);
  }
});

test("entailMaterialize rejects an unknown regime, naming the accepted set", () => {
  assert.throws(() => entailMaterialize(SCHEMA, "rdfs-plus"), /accepted: simple, rdf, rdfs/);
  // The spellings are case-sensitive, exactly as the CLI writes them.
  assert.throws(() => entailMaterialize(SCHEMA, "RDFS"), /accepted:/);
});

test("entailMaterialize refuses the three non-materializable regimes by name", () => {
  for (const regime of ["owl-direct", "rif", "d"]) {
    assert.throws(
      () => entailMaterialize(SCHEMA, regime),
      /materializable regimes: simple, rdf, rdfs, owl-rl/,
      regime,
    );
  }
});

test("entailMaterialize rejects a malformed document (never a silent empty closure)", () => {
  assert.throws(() => entailMaterialize("this is not n-quads\n", "rdfs"));
});

test("the rule inventories are the specification tables, and the gap is measurable", () => {
  // OWL 2 Profiles §4.3 Tables 4-9; RDF 1.1 Semantics §9.2.1.
  assert.equal(entailRules("owl-rl").length, 78);
  assert.equal(entailRules("rdfs").length, 18);
  assert.equal(entailRules("rdf").length, 3);
  // A regime with no rule table is an empty array, not `[""]`.
  assert.deepEqual(entailRules("simple"), []);
  assert.deepEqual(entailImplementedRules("simple"), []);

  // The implemented set is a strict subsequence of the defined set…
  for (const regime of ["rdf", "rdfs", "owl-rl"]) {
    const defined = entailRules(regime);
    const fired = entailImplementedRules(regime);
    assert.ok(fired.length < defined.length, `${regime}: the gap must be visible`);
    for (const rule of fired) assert.ok(defined.includes(rule), `${regime}: ${rule}`);
  }

  // …and the difference is exactly what the report's `missing` lines name.
  const missing = entailMaterialize(SCHEMA, "rdfs")
    .report.split("\n")
    .filter((line) => line.startsWith("missing "))
    .map((line) => line.slice("missing ".length));
  const defined = entailRules("rdfs");
  const fired = entailImplementedRules("rdfs");
  assert.deepEqual(
    missing,
    defined.filter((rule) => !fired.includes(rule)),
  );
});

test("the rule inventories reject an unknown regime, naming the accepted set", () => {
  assert.throws(() => entailRules("rdfs-plus"), /accepted: simple, rdf, rdfs/);
  assert.throws(() => entailImplementedRules("rdfs-plus"), /accepted: simple, rdf, rdfs/);
});
