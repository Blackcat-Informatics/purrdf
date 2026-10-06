// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: exercises the published JavaScript wrapper's surface — the divisionPolicy property, the thrown coded error and the wrapped expressionErrors evidence map — which exists only in JavaScript

// The precision of an xsd:integer/xsd:decimal quotient through the built package, and
// the F&O expression errors a governed query absorbed. Each refusal sits beside the
// valid neighbour it must keep.

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, Dataset, QueryEngine, expressionErrorCodes } from "../index.mjs";

await ready();

const SEED = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
const seed = () => Dataset.parse(SEED, "ntriples");

const CODES = [
  "err:FOAR0001",
  "err:FOAR0002",
  "err:FOCA0001",
  "err:FOCA0002",
  "err:FOCA0003",
  "err:FOCA0006",
  "err:FORG0001",
  "err:XPTY0004",
];

function quotient(engine, text) {
  const rows = engine.select(seed(), `SELECT (${text} AS ?x) {}`).rows.toArray();
  assert.equal(rows.length, 1);
  return rows[0].x.value;
}

function insertQuotient(text) {
  return `INSERT { <http://example.org/s> <http://example.org/q> ?x } WHERE { BIND(${text} AS ?x) }`;
}

function zeroes() {
  return Object.fromEntries(CODES.map((code) => [code, 0n]));
}

test("an engine starts at eighteen digits truncated toward zero", () => {
  const engine = new QueryEngine();
  assert.equal(engine.divisionPolicy, "18:toward-zero");
  assert.equal(quotient(engine, "2/3"), "0.666666666666666666");
});

test("exact answers a terminating quotient and refuses a non-terminating one", () => {
  const engine = new QueryEngine();
  engine.divisionPolicy = "exact";
  assert.equal(engine.divisionPolicy, "exact");
  assert.equal(quotient(engine, "1/8"), "0.125");
  assert.throws(
    () => quotient(engine, "1/3"),
    (error) => error.code === "native-sparql-numeric" && /FOAR0002/.test(error.message),
  );
});

test("a rounded policy rounds in its named direction", () => {
  const engine = new QueryEngine();
  engine.divisionPolicy = "5:half-even";
  assert.equal(engine.divisionPolicy, "5:half-even");
  assert.equal(quotient(engine, "2/3"), "0.66667");
});

test("an unreadable policy is refused and the previous one stays in force", () => {
  const engine = new QueryEngine();
  engine.divisionPolicy = "5:half-even";
  for (const refused of ["5:sideways", "", "inexact", "-1"]) {
    assert.throws(
      () => {
        engine.divisionPolicy = refused;
      },
      (error) => error.code === "purrdf-wasm-options" && /division policy/.test(error.message),
      refused,
    );
    assert.equal(engine.divisionPolicy, "5:half-even");
  }
  assert.equal(quotient(engine, "2/3"), "0.66667");
  engine.divisionPolicy = "7";
  assert.equal(engine.divisionPolicy, "7:toward-zero");
});

test("the policy reaches the governed, update and entailment entries", () => {
  const engine = new QueryEngine();
  engine.divisionPolicy = "exact";
  const refusesFoar0002 = (error) =>
    error.code === "native-sparql-numeric" && /FOAR0002/.test(error.message);

  const governed = engine.queryGoverned(seed(), "SELECT (1/8 AS ?x) {}");
  assert.equal(governed.isComplete, true);
  assert.equal(governed.result.rows.toArray()[0].x.value, "0.125");
  assert.throws(() => engine.queryGoverned(seed(), "SELECT (1/3 AS ?x) {}"), refusesFoar0002);

  const target = seed();
  assert.throws(() => engine.updateGoverned(target, insertQuotient("1/3")), refusesFoar0002);
  assert.equal(target.size, 1, "a refused update applies nothing");
  assert.equal(engine.updateGoverned(target, insertQuotient("1/8")).isApplied, true);
  assert.equal(target.size, 2);

  assert.throws(() => engine.update(seed(), insertQuotient("1/3")), refusesFoar0002);
  assert.equal(engine.update(seed(), insertQuotient("1/8")).size, 2);

  const entailed = engine.queryEntailmentGoverned(seed(), "SELECT (1/8 AS ?x) {}", "rdfs");
  assert.equal(entailed.outcome.result.rows.toArray()[0].x.value, "0.125");
  assert.throws(
    () => engine.queryEntailmentGoverned(seed(), "SELECT (1/3 AS ?x) {}", "rdfs"),
    refusesFoar0002,
  );
});

test("governed evidence counts each absorbed expression error by its F&O code", () => {
  assert.deepEqual(expressionErrorCodes(), CODES);
  const engine = new QueryEngine();
  const absorbed = engine.queryGoverned(seed(), "SELECT (1/0 AS ?x) {}");
  assert.equal(absorbed.isComplete, true);
  assert.deepEqual(Object.keys(absorbed.evidence.expressionErrors), CODES);
  assert.deepEqual({ ...absorbed.evidence.expressionErrors }, { ...zeroes(), "err:FOAR0001": 1n });

  const valid = engine.queryGoverned(seed(), "SELECT (1/2 AS ?x) {}");
  assert.deepEqual({ ...valid.evidence.expressionErrors }, zeroes());

  const update = engine.updateGoverned(seed(), insertQuotient("1/0"));
  assert.equal(update.isApplied, true);
  assert.equal(update.evidence.expressionErrors["err:FOAR0001"], 1n);
});

test("an asynchronous job takes the policy and reports its expression errors", async () => {
  const engine = new QueryEngine();
  engine.divisionPolicy = "exact";
  const answered = await engine.queryGovernedAsync(seed(), "SELECT (1/8 AS ?x) {}");
  assert.equal(answered.result.rows.toArray()[0].x.value, "0.125");
  assert.deepEqual({ ...answered.evidence.async.expressionErrors }, zeroes());
  await assert.rejects(engine.queryAsync(seed(), "SELECT (1/3 AS ?x) {}"), /FOAR0002/);

  const absorbed = await engine.queryGovernedAsync(seed(), "SELECT (1/0 AS ?x) {}");
  assert.equal(absorbed.evidence.expressionErrors["err:FOAR0001"], 1n);
  assert.equal(absorbed.evidence.async.expressionErrors["err:FOAR0001"], 1n);
});
