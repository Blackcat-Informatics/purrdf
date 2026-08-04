// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Node real-execution conformance for the GOVERNED SPARQL surface
// (QueryEngine.queryGoverned / updateGoverned / explainQuery), driven against the ACTUAL
// optimized wasm module rather than a native build of the same Rust.
//
// That distinction is the reason this file exists. The engine's wall deadline is written
// per target: `std::time::Instant` on native, `js_sys::Date::now()` on wasm32. The native
// spelling COMPILES for wasm32 and panics at run time, so a green native test suite and a
// green `make wasm` build together prove nothing about whether a deadline works in a
// browser. Only executing one inside a real module does. `deadlineMs` is therefore
// exercised twice below — once at zero, which must trip on the first stop poll, and once
// at a non-zero budget over a query that cannot possibly finish inside it, which additionally
// proves the clock ADVANCES rather than merely being readable.
//
// Timing is never asserted. The machine running this is not quiet, and a loaded machine
// only makes a deadline trip sooner in relative terms; what is asserted is that the trip
// HAPPENED and that it named the deadline.

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, CancellationToken, Dataset, QueryEngine, governorDimensions } from "../index.mjs";

// One-time wasm instantiation before any test runs.
await ready();

// The ceiling a METERED-but-unbounded dimension carries: engaged, so the counter runs, and
// one below the largest representable, so nothing an execution can consume reaches it.
const METERED_CEILING = 2n ** 64n - 2n;

const TRIG = `
@prefix ex: <https://example.org/> .
ex:a ex:knows ex:b .
ex:a ex:name "Ann" .
ex:b ex:name "Bob" .
`;

// Two solution rows, deterministically ordered — the fixture the answer-cap boundary is
// measured against.
const NAMES = "PREFIX ex: <https://example.org/> SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name";

const RDFS = `
@prefix ex: <https://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
ex:Cat rdfs:subClassOf ex:Animal .
ex:tom rdf:type ex:Cat .
`;
const RDFS_QUERY = `
PREFIX ex: <https://example.org/>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
SELECT ?x WHERE { ?x rdf:type ex:Animal }
`;

// A value-CONSTRUCTING query: the only shape that mints bytes into the per-query scratch
// arena, and therefore the only one a scratch ceiling can bind on.
const CONCAT =
  'PREFIX ex: <https://example.org/> SELECT (CONCAT(?name, "!") AS ?greeting) WHERE { ?p ex:name ?name }';

// An RDF 1.2 asset: a reified statement (`~ ex:r`) and a triple term asserted as an
// object. Both are first-class here, not an extension.
const RDF12 = `
@prefix ex: <https://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
ex:a ex:knows ex:b ~ ex:r .
ex:r ex:statedBy ex:ann .
ex:claim rdf:reifies <<( ex:c ex:knows ex:d )>> .
`;

// A query no wall-clock budget of a few dozen milliseconds can cover: a three-way cross
// product over the fixture below, folded through an aggregate so the ANSWER is one row and
// the cost is entirely in the join. Nothing else can trip it — every other dimension is
// metered at a ceiling no execution reaches — so a trip here is the deadline or nothing.
const CROSS_PRODUCT =
  "SELECT (COUNT(*) AS ?n) WHERE { ?a ?b ?c . ?d ?e ?f . ?g ?h ?i }";

function wideDataset(size) {
  const lines = ["@prefix ex: <https://example.org/> ."];
  for (let index = 0; index < size; index += 1) {
    lines.push(`ex:s${index} ex:p ex:o${index} .`);
  }
  return Dataset.parse(lines.join("\n"), "turtle");
}

function names(outcome) {
  return outcome.result.rows.toArray().map((row) => row.name.value);
}

test("an ungoverned query is unchanged: same rows, and no governor vocabulary reaches it", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");

  const before = engine.select(ds, NAMES).rows.toArray().map((row) => row.name.value);
  assert.deepEqual(before, ["Ann", "Bob"]);

  // A governed call in between must not perturb the ungoverned lane.
  engine.queryGoverned(ds, NAMES, { maxAnswers: 1 });

  const after = engine.select(ds, NAMES).rows.toArray().map((row) => row.name.value);
  assert.deepEqual(after, before);
  assert.deepEqual(
    JSON.parse(ds.query(NAMES)).results.bindings.map((b) => b.name.value),
    ["Ann", "Bob"],
  );
});

test("an ungoverned entry refuses a governor option rather than ignoring it", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  for (const options of [
    { fuel: 1 },
    { deadlineMs: 0 },
    { maxAnswers: 1 },
    { maxIntermediateCells: 1 },
    { maxScratchBytes: 1 },
    { maxRemoteRequests: 1 },
    { cancel: new CancellationToken() },
  ]) {
    assert.throws(() => engine.select(ds, NAMES, options), TypeError);
    assert.throws(() => engine.query(ds, NAMES, options), TypeError);
    assert.throws(() => engine.queryRaw(ds, NAMES, options), TypeError);
  }
});

test("queryGoverned with no ceiling completes and still hands back a receipt", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const outcome = engine.queryGoverned(ds, NAMES);

  assert.equal(outcome.isComplete, true);
  assert.equal(outcome.tripped, undefined);
  assert.equal(outcome.partial, undefined);
  assert.equal(outcome.result.kind, "select");
  assert.deepEqual(names(outcome), ["Ann", "Bob"]);

  // Metered, not bounded: every caller-settable dimension carries a ceiling no execution
  // can reach, and the counters ran, which is what makes the next budget sizeable.
  assert.equal(outcome.evidence.isComplete, true);
  assert.equal(outcome.evidence.limits.fuel, METERED_CEILING);
  assert.equal(outcome.evidence.limits["answer-rows"], METERED_CEILING);
  assert.ok(outcome.evidence.consumed.fuel > 0n, "a metered query must charge fuel");
  assert.equal(outcome.evidence.consumed["answer-rows"], 2n);
});

test("queryEntailmentGoverned carries the query outcome and closure report together", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(RDFS, "turtle");
  const outcome = engine.queryEntailmentGoverned(ds, RDFS_QUERY, "rdfs");

  assert.equal(outcome.phase, "answered");
  assert.equal(outcome.isComplete, true);
  assert.equal(outcome.tripped, undefined);
  assert.match(outcome.report, /^purrdf-reasoning-report 4\n/);
  assert.match(outcome.report, /\nregime rdfs\n/);
  assert.equal(outcome.outcome.isComplete, true);
  assert.equal(outcome.outcome.result.kind, "select");
  assert.equal(outcome.outcome.result.rowCount, 1);
});

test("queryEntailmentGoverned exposes a closure stop without an answer or report", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(RDFS, "turtle");
  const outcome = engine.queryEntailmentGoverned(ds, RDFS_QUERY, "rdfs", {
    deadlineMs: 0,
  });

  assert.equal(outcome.phase, "closure-stopped");
  assert.equal(outcome.isComplete, false);
  assert.equal(outcome.outcome, undefined);
  assert.equal(outcome.report, undefined);
  assert.equal(outcome.tripped.cause, "deadline-exceeded");
});

test("the evidence maps are keyed by the engine's own dimension vocabulary", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const dimensions = governorDimensions();
  assert.ok(dimensions.includes("fuel"));
  assert.ok(dimensions.includes("answer-rows"));

  const { evidence } = engine.queryGoverned(ds, NAMES);
  assert.deepEqual(Object.keys(evidence.consumed), dimensions);
  assert.deepEqual(Object.keys(evidence.limits), dimensions);
});

test("the answer cap is inclusive: cap == answer size completes", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const outcome = engine.queryGoverned(ds, NAMES, { maxAnswers: 2 });

  assert.equal(outcome.isComplete, true);
  assert.equal(outcome.tripped, undefined);
  assert.deepEqual(names(outcome), ["Ann", "Bob"]);
  assert.equal(outcome.evidence.limits["answer-rows"], 2n);
});

test("the answer cap is inclusive: cap == answer size - 1 exhausts and certifies what it reached", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const outcome = engine.queryGoverned(ds, NAMES, { maxAnswers: 1 });

  assert.equal(outcome.isComplete, false);
  assert.equal(outcome.result, undefined, "a truncated answer never wears the complete type");
  assert.equal(outcome.tripped.kind, "budget");
  assert.equal(outcome.tripped.label, "answer-cap-exhausted");
  assert.equal(outcome.tripped.dimension, "answer-rows");
  assert.equal(outcome.tripped.limit, 1n);
  assert.equal(outcome.tripped.cause, undefined);
  assert.equal(outcome.tripped.estimate, undefined);

  assert.equal(outcome.partial.certainty, "certain");
  assert.equal(outcome.partial.isCertain, true);
  assert.equal(typeof outcome.partial.isPositionalPrefix, "boolean");
  assert.equal(outcome.partial.barrier, undefined);
  assert.deepEqual(outcome.partial.result.rows.toArray().map((row) => row.name.value), ["Ann"]);

  assert.equal(outcome.evidence.isComplete, false);
  assert.equal(outcome.evidence.limits["answer-rows"], 1n);
});

test("a zero fuel ceiling trips on the first charged unit of work", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const outcome = engine.queryGoverned(ds, NAMES, { fuel: 0 });

  assert.equal(outcome.isComplete, false);
  assert.equal(outcome.tripped.kind, "budget");
  assert.equal(outcome.tripped.label, "fuel-exhausted");
  assert.equal(outcome.tripped.dimension, "fuel");
  assert.equal(outcome.tripped.limit, 0n);
  assert.equal(typeof outcome.tripped.consumed, "bigint");
  assert.ok(outcome.partial !== undefined, "a trip always carries a certificate");
});

test("a fuel ceiling large enough to cover the query completes it", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const sized = engine.queryGoverned(ds, NAMES).evidence.consumed.fuel;
  const outcome = engine.queryGoverned(ds, NAMES, { fuel: sized });

  assert.equal(outcome.isComplete, true, "the metered figure must be a sufficient budget");
  assert.deepEqual(names(outcome), ["Ann", "Bob"]);
});

// ---------------------------------------------------------------------------
// The wall deadline, executed in a real wasm module. See the file header.
// ---------------------------------------------------------------------------

test("a zero wall deadline trips on the first stop poll inside the real wasm module", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");

  // If the deadline clock were still `std::time::Instant`, CONSTRUCTING the deadline would
  // panic here and this call would throw rather than return an outcome.
  let outcome;
  assert.doesNotThrow(() => {
    outcome = engine.queryGoverned(ds, NAMES, { deadlineMs: 0 });
  }, "reading the wasm wall clock must not panic");

  assert.equal(outcome.isComplete, false);
  assert.equal(outcome.tripped.kind, "stopped");
  assert.equal(outcome.tripped.label, "deadline-exceeded");
  assert.equal(outcome.tripped.cause, "deadline-exceeded");
  assert.equal(outcome.tripped.dimension, undefined, "a stop signal belongs to no dimension");
  assert.equal(outcome.tripped.limit, undefined);
  assert.ok(outcome.partial !== undefined);
});

test("a non-zero wall deadline trips a query it cannot cover, so the wasm clock advances", () => {
  const engine = new QueryEngine();
  // ~1.7 million three-way combinations: far more work than any few-dozen-millisecond
  // budget covers, on any machine, however lightly or heavily loaded.
  const ds = wideDataset(120);

  const outcome = engine.queryGoverned(ds, CROSS_PRODUCT, { deadlineMs: 50 });

  assert.equal(outcome.isComplete, false, "a 50ms budget cannot cover a 1.7M-row cross product");
  assert.equal(outcome.tripped.label, "deadline-exceeded");
  assert.equal(outcome.tripped.kind, "stopped");
  // A zero deadline would trip on the snapshot alone; this one can only trip because a
  // LATER read of js_sys::Date::now() returned a larger value than the one at construction.
  assert.ok(outcome.evidence.consumed.fuel > 0n, "the query must have run before it stopped");
});

test("the same query completes when the wall deadline is not the binding constraint", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  // A generous budget over a three-row fixture: the deadline must not fire, which is what
  // proves the zero-budget trip above was the deadline doing its job rather than the
  // signal being stuck on.
  const outcome = engine.queryGoverned(ds, NAMES, { deadlineMs: 600_000 });

  assert.equal(outcome.isComplete, true);
  assert.equal(outcome.tripped, undefined);
  assert.deepEqual(names(outcome), ["Ann", "Bob"]);
});

// ---------------------------------------------------------------------------
// Cancellation
// ---------------------------------------------------------------------------

test("a cancelled token stops the query as an outcome and survives the call", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const cancel = new CancellationToken();
  assert.equal(cancel.isCancelled, false);
  cancel.cancel();
  assert.equal(cancel.isCancelled, true);

  const outcome = engine.queryGoverned(ds, NAMES, { cancel });
  assert.equal(outcome.isComplete, false);
  assert.equal(outcome.tripped.kind, "stopped");
  assert.equal(outcome.tripped.label, "cancelled");
  assert.equal(outcome.tripped.cause, "cancelled");

  // The handle the caller holds is still alive and still governs: a governed call is
  // handed a SHARE of the bit, not the caller's own token.
  assert.equal(cancel.isCancelled, true);
  const again = engine.queryGoverned(ds, NAMES, { cancel });
  assert.equal(again.tripped.label, "cancelled");
});

test("an uncancelled token leaves the query complete", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const cancel = new CancellationToken();
  const outcome = engine.queryGoverned(ds, NAMES, { cancel });

  assert.equal(outcome.isComplete, true);
  assert.deepEqual(names(outcome), ["Ann", "Bob"]);
  assert.equal(cancel.isCancelled, false);
});

test("cancellation outranks a deadline when both fire at the same point", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const cancel = new CancellationToken();
  cancel.cancel();

  const outcome = engine.queryGoverned(ds, NAMES, { cancel, deadlineMs: 0 });
  assert.equal(outcome.tripped.label, "cancelled", "an explicit decision outranks an elapsed measurement");
});

// ---------------------------------------------------------------------------
// A trip is returned, never thrown
// ---------------------------------------------------------------------------

test("every governor trip is a returned outcome, not a throw", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const cancelled = new CancellationToken();
  cancelled.cancel();

  // Each ceiling is paired with a query it can actually bind on: a governor that cannot
  // trip proves nothing about how a trip is delivered.
  for (const [sparql, options] of [
    [NAMES, { fuel: 0 }],
    [NAMES, { maxAnswers: 0 }],
    [NAMES, { maxIntermediateCells: 0 }],
    [CONCAT, { maxScratchBytes: 0 }],
    [NAMES, { deadlineMs: 0 }],
    [NAMES, { cancel: cancelled }],
  ]) {
    let outcome;
    assert.doesNotThrow(() => {
      outcome = engine.queryGoverned(ds, sparql, options);
    }, `a trip under ${JSON.stringify(Object.keys(options))} must not throw`);
    assert.equal(outcome.isComplete, false);
    assert.equal(typeof outcome.tripped.label, "string");
    assert.equal(outcome.evidence.isComplete, false);
  }
});

test("a ceiling the planner's estimate already exceeds is REFUSED, and reports an estimate rather than a measurement", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const outcome = engine.queryGoverned(ds, NAMES, { maxIntermediateCells: 0 });

  assert.equal(outcome.isComplete, false);
  assert.equal(outcome.tripped.kind, "refused");
  assert.equal(outcome.tripped.label, "cardinality-admission-refused");
  assert.equal(outcome.tripped.dimension, "intermediate-cells");
  assert.equal(outcome.tripped.limit, 0n);
  assert.equal(typeof outcome.tripped.estimate, "bigint");
  assert.equal(
    outcome.tripped.consumed,
    undefined,
    "nothing ran, so there is nothing to have measured",
  );
  assert.equal(outcome.evidence.consumed["intermediate-cells"], 0n);
});

test("the scratch ceiling binds a value-constructing query and leaves a plain one alone", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");

  // A query that mints nothing into the scratch arena is complete even at a zero ceiling:
  // the boundary is inclusive, and zero consumption is within a zero budget.
  const plain = engine.queryGoverned(ds, NAMES, { maxScratchBytes: 0 });
  assert.equal(plain.isComplete, true);
  assert.equal(plain.evidence.consumed["scratch-bytes"], 0n);

  const constructing = engine.queryGoverned(ds, CONCAT, { maxScratchBytes: 0 });
  assert.equal(constructing.isComplete, false);
  assert.equal(constructing.tripped.kind, "budget");
  assert.equal(constructing.tripped.label, "scratch-exhausted");
  assert.equal(constructing.tripped.dimension, "scratch-bytes");
});

test("a genuine query error still throws — a trip is not the only outcome shape", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  assert.throws(() => engine.queryGoverned(ds, "SELECT ?x WHERE { this is not sparql"));
  assert.throws(() =>
    engine.queryGoverned(
      ds,
      "PREFIX ex: <https://example.org/> SELECT ?o WHERE { SERVICE <https://remote.example.org/sparql> { ?s ex:knows ?o } }",
    ),
  );
});

test("a negative ceiling is refused rather than wrapping into an unreachable one", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  for (const key of [
    "fuel",
    "deadlineMs",
    "maxAnswers",
    "maxIntermediateCells",
    "maxScratchBytes",
    "maxRemoteRequests",
  ]) {
    assert.throws(
      () => engine.queryGoverned(ds, NAMES, { [key]: -1 }),
      new Error(
        `governor ceiling \`${key}\` must be a non-negative integer, got -1 ` +
          "(omit it to decline the ceiling; 0 is a valid ceiling that trips on the " +
          "first charged unit of work)",
      ),
      `a negative ${key} must be refused`,
    );
  }
  assert.throws(() => engine.queryGoverned(ds, NAMES, { fuel: 1.5 }), TypeError);
  for (const value of ["", "   ", [], true, false]) {
    assert.throws(
      () => engine.queryGoverned(ds, NAMES, { fuel: value }),
      TypeError,
      `${JSON.stringify(value)} must not be silently coerced into an integer ceiling`,
    );
  }
});

// ---------------------------------------------------------------------------
// RDF 1.2 is first class on the governed lane too
// ---------------------------------------------------------------------------

test("the answer cap counts RDF 1.2 triple-term solutions like any other", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(RDF12, "turtle");
  const triples =
    "PREFIX ex: <https://example.org/> PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> " +
    "SELECT ?s ?p ?o WHERE { ex:claim rdf:reifies <<( ?s ?p ?o )>> }";

  const complete = engine.queryGoverned(ds, triples);
  assert.equal(complete.isComplete, true);
  assert.equal(complete.result.rowCount, 1);
  assert.equal(complete.evidence.consumed["answer-rows"], 1n);

  const capped = engine.queryGoverned(ds, triples, { maxAnswers: 0 });
  assert.equal(capped.isComplete, false);
  assert.equal(capped.tripped.label, "answer-cap-exhausted");
});

test("the answer cap counts RDF 1.2 reifier statements in a CONSTRUCT answer sequence", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(RDF12, "turtle");
  // The reified statement contributes its reifier binding as well as the base triple, so
  // this CONSTRUCT's answer sequence is statements, not solution rows.
  const construct =
    "PREFIX ex: <https://example.org/> CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }";

  const complete = engine.queryGoverned(ds, construct);
  assert.equal(complete.isComplete, true);
  assert.equal(complete.result.kind, "graph");
  const statements = complete.evidence.consumed["answer-rows"];
  assert.ok(statements > 0n, "a graph form must charge its OUTPUT STATEMENTS, not its rows");

  const capped = engine.queryGoverned(ds, construct, { maxAnswers: statements - 1n });
  assert.equal(capped.isComplete, false, "one statement below the produced size must trip");
  assert.equal(capped.tripped.dimension, "answer-rows");

  const exact = engine.queryGoverned(ds, construct, { maxAnswers: statements });
  assert.equal(exact.isComplete, true, "the boundary is inclusive on a graph form too");
});

// ---------------------------------------------------------------------------
// Governed UPDATE
// ---------------------------------------------------------------------------

test("a governed UPDATE that fits its budget applies and reports a receipt", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const before = ds.size;

  const outcome = engine.updateGoverned(
    ds,
    "INSERT DATA { <https://example.org/c> <https://example.org/p> <https://example.org/d> }",
  );

  assert.equal(outcome.isApplied, true);
  assert.equal(outcome.tripped, undefined);
  assert.equal(outcome.evidence.isComplete, true);
  assert.equal(ds.size, before + 1);
});

test("a tripped UPDATE applies NOTHING and leaves the dataset byte-identical", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const before = ds.canonicalize();
  const size = ds.size;

  const outcome = engine.updateGoverned(
    ds,
    "PREFIX ex: <https://example.org/> DELETE { ?s ?p ?o } INSERT { ?s ex:seen true } WHERE { ?s ?p ?o }",
    { fuel: 0 },
  );

  assert.equal(outcome.isApplied, false);
  assert.equal(outcome.tripped.label, "fuel-exhausted");
  assert.equal(outcome.evidence.isComplete, false);
  assert.equal(ds.size, size);
  assert.equal(ds.canonicalize(), before, "a tripped request is not a partial mutation");
});

test("a cancelled token stops an UPDATE as an outcome", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const before = ds.canonicalize();
  const cancel = new CancellationToken();
  cancel.cancel();

  const outcome = engine.updateGoverned(
    ds,
    "PREFIX ex: <https://example.org/> INSERT { ?s ex:seen true } WHERE { ?s ?p ?o }",
    { cancel },
  );

  assert.equal(outcome.isApplied, false);
  assert.equal(outcome.tripped.label, "cancelled");
  assert.equal(ds.canonicalize(), before);
});

test("updateGoverned refuses maxAnswers rather than ignoring a ceiling the caller set", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  assert.throws(
    () =>
      engine.updateGoverned(
        ds,
        "INSERT DATA { <https://example.org/c> <https://example.org/p> <https://example.org/d> }",
        { maxAnswers: 1 },
      ),
    /maxAnswers/,
  );
  assert.equal(ds.size, 3, "the refused request must not have run");
});

// ---------------------------------------------------------------------------
// Sizing a budget
// ---------------------------------------------------------------------------

test("explainQuery renders the charge ledger a budget is sized from", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const explanation = engine.explainQuery(ds, NAMES);

  assert.match(explanation, /^profile purrdf-sparql-governors v\d+ digest [0-9a-f]+ /);
  assert.match(explanation, /\nschedule\n/);
  assert.match(explanation, /\nledger\n/);
  assert.match(explanation, /\njoin-orders\n/);
  assert.match(explanation, /\nconsumed\n/);
  assert.match(explanation, /\n {2}fuel\t\d+\n/);

  // Deterministic: a ledger with a clock or an address in it would not repeat.
  assert.equal(engine.explainQuery(ds, NAMES), explanation);
});
