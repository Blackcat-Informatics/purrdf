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
  entailCheckInconsistentRefusal,
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

test("an inconsistent input is refused WITH its certificate ON WASM", () => {
  // The path the golden artifact cannot cover: an inconsistent knowledge base has no
  // closure, so the only channel the evidence has is the thrown message. Same checker
  // as the C-ABI and `purrdf-validate` tests.
  entailCheckInconsistentRefusal();
  // …and a caller who reaches the boundary directly sees the same bytes: the witness
  // rule, the graph whose closure refused, and the three asserted triples.
  assert.throws(
    () =>
      entailMaterialize(
        [
          "<http://example.org/A> <http://www.w3.org/2002/07/owl#disjointWith> <http://example.org/B> .",
          "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .",
          "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B> .",
          "",
        ].join("\n"),
        "owl-rl",
        "",
      ),
    /inconsistency-premise <http:\/\/example\.org\/A>/,
  );
});

// The same artifact, parsed here and driven case by case through the JS boundary — so
// the comparison covers the wasm→JS string marshalling too, not only the byte equality
// the checker establishes inside the module.
const VECTORS = fileURLToPath(
  new URL("../../../validate/tests/fixtures/regime-boundary.vectors", import.meta.url),
);

/** Parse the line-oriented `@case/@regime/@input/@program/@closure/@report/@end` artifact. */
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
    else if (
      keyword === "input" ||
      keyword === "program" ||
      keyword === "closure" ||
      keyword === "report"
    ) {
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
    const closed = entailMaterialize(vector.input, vector.regime, vector.program ?? "");
    assert.equal(closed.nquads, vector.closure, `${vector.name}: closure`);
    assert.equal(closed.report, vector.report, `${vector.name}: report`);
  }
});

test("entailMaterialize closes under rdfs and always returns a report", () => {
  const closed = entailMaterialize(SCHEMA, "rdfs", "");
  assert.match(
    closed.nquads,
    /<http:\/\/example\.org\/x> <http:\/\/www\.w3\.org\/1999\/02\/22-rdf-syntax-ns#type> <http:\/\/example\.org\/C> \./,
  );
  assert.ok(closed.report.startsWith("purrdf-reasoning-report 2\n"));
  assert.ok(closed.report.includes("\nregime rdfs\n"));
  // The report says what the run could NOT do, rather than claiming completeness
  // it does not have. Asserted as the invariant, not as a `sound-incomplete <n>`
  // literal: the count moves as rules land, and a `boundary` line outlives a rule
  // table going complete.
  assert.ok(closed.report.includes("\ncompleteness "));
  assert.ok(closed.report.includes("\nboundary "));
  // The only observable rdfD1 / rdfD1a / rdfs14 / rdfs14a have: all four fire and none
  // of their conclusions can reach a `fired` line. It reached the command line only,
  // which left them invisible from exactly the hosts the report exists for.
  assert.ok(closed.report.includes("\nwithheld-surrogates "));
  assert.ok(closed.report.endsWith("inconsistency none\n"));
});

test("entailMaterialize under simple is the identity closure", () => {
  const closed = entailMaterialize(SCHEMA, "simple", "");
  assert.ok(
    !closed.nquads.includes(
      "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .",
    ),
  );
  assert.ok(closed.report.includes("\ncompleteness exact\n"));
});

test("entailMaterialize is byte-stable across repeated calls", () => {
  const first = entailMaterialize(SCHEMA, "owl-rl", "");
  for (let i = 0; i < 5; i += 1) {
    const again = entailMaterialize(SCHEMA, "owl-rl", "");
    assert.equal(again.nquads, first.nquads);
    assert.equal(again.report, first.report);
  }
});

test("entailMaterialize rejects an unknown regime, naming the accepted set", () => {
  assert.throws(() => entailMaterialize(SCHEMA, "rdfs-plus", ""), /accepted: simple, rdf, rdfs/);
  // The spellings are case-sensitive, exactly as the CLI writes them.
  assert.throws(() => entailMaterialize(SCHEMA, "RDFS", ""), /accepted:/);
});

// A normative RIF-in-XML rule document: `?x a ex:A` => `?x a ex:B`. `rif` is the
// one regime whose calculus is the CALLER's, so it is the one spelling whose
// `program` argument is a document rather than the empty string.
const RIF_PROGRAM =
  '<Document xmlns="http://www.w3.org/2007/rif#"><payload><Group><sentence><Forall><declare><Var>x</Var></declare><formula><Implies><if><Frame><object><Var>x</Var></object><slot><Const type="http://www.w3.org/2007/rif#iri">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type="http://www.w3.org/2007/rif#iri">http://example.org/A</Const></slot></Frame></if><then><Frame><object><Var>x</Var></object><slot><Const type="http://www.w3.org/2007/rif#iri">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type="http://www.w3.org/2007/rif#iri">http://example.org/B</Const></slot></Frame></then></Implies></formula></Forall></sentence></Group></payload></Document>';

test("entailMaterialize materializes every regime spelling", () => {
  // Falsifiable against the old behavior: `owl-direct` and `rif` threw here with a
  // message naming the five spellings that were not refused.
  for (const [regime, program] of [
    ["simple", ""],
    ["rdf", ""],
    ["rdfs", ""],
    ["owl-rl", ""],
    ["owl-direct", ""],
    ["rif", RIF_PROGRAM],
    ["d", ""],
  ]) {
    const closed = entailMaterialize(SCHEMA, regime, program);
    assert.match(closed.report, /^purrdf-reasoning-report 2\n/);
    assert.ok(closed.report.includes(`\nregime ${regime}\n`), regime);
    assert.ok(closed.report.includes("\nwithheld-surrogates "), regime);
    assert.ok(closed.report.endsWith("inconsistency none\n"), regime);
  }
});

test("a rule document belongs to rif alone and is refused elsewhere", () => {
  assert.throws(
    () => entailMaterialize(SCHEMA, "rdfs", RIF_PROGRAM),
    /takes no rule document/,
  );
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

  // The implemented set is a subsequence of the defined set — no additions, and
  // the gap is legitimately empty for a regime whose table is fully implemented…
  for (const regime of ["rdf", "rdfs", "owl-rl", "d"]) {
    const defined = entailRules(regime);
    const fired = entailImplementedRules(regime);
    assert.ok(fired.length <= defined.length, `${regime}: no rule is invented`);
    for (const rule of fired) assert.ok(defined.includes(rule), `${regime}: ${rule}`);
  }

  // …and the difference is exactly what the report's `missing` lines name.
  const missing = entailMaterialize(SCHEMA, "rdfs", "")
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
