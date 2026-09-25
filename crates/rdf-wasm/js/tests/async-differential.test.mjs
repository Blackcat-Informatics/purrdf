// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// The differential oracle for the asynchronous lane. Every query of the fixture runs
// synchronously (the oracle) and through `queryAsync` with `yieldEveryPolls: 0` — a
// suspension at EVERY governor poll — eight jobs at a time on one instance, so their
// evaluations interleave at the finest grain the runtime has. Each asynchronous answer
// must equal the synchronous one.
//
// This is what makes the runtime's interleaving audit executable: a lock, a `RefCell`
// borrow or a last-in-first-out guard that some evaluator path held across a poll would
// be observed half-held by the next job to resume — a panic (a trap) or a wrong answer
// here, not a silent pass.

import { test } from "node:test";
import assert from "node:assert/strict";

import { Dataset, QueryEngine, ready } from "../index.mjs";
import init from "../pkg/purrdf_wasm.js";
import { DATASETS, QUERIES } from "./fixtures/async-differential.mjs";

await ready();
const exports = await init();
const stackPointer = () => exports.__wbindgen_add_to_stack_pointer(0) >>> 0;

const CONCURRENCY = 8;

/** A term in N-Triples-like spelling, RDF 1.2 triple terms and directions included. */
function termKey(term) {
  switch (term.termType) {
    case "NamedNode":
      return `<${term.value}>`;
    case "BlankNode":
      return `_:${term.value}`;
    case "Literal":
      if (term.language !== "") {
        return `${JSON.stringify(term.value)}@${term.language}${term.direction === "" ? "" : `--${term.direction}`}`;
      }
      return `${JSON.stringify(term.value)}^^<${term.datatype.value}>`;
    case "Quad":
      return `<<( ${termKey(term.subject)} ${termKey(term.predicate)} ${termKey(term.object)} )>>`;
    default:
      throw new Error(`unexpected term type ${term.termType}`);
  }
}

/**
 * A result, canonicalised: SELECT rows as `var=term` strings in answer order AND sorted
 * (the order compared exactly when the query orders, the multiset always); ASK as its
 * boolean; a graph by its canonical N-Quads.
 */
function canonical(result, ordered) {
  switch (result.kind) {
    case "select": {
      const rows = result.rows.toArray().map((row) =>
        result.variables.map((name) => `${name}=${row[name] === undefined ? "UNBOUND" : termKey(row[name])}`).join(" "),
      );
      return {
        kind: "select",
        variables: result.variables,
        rows: ordered ? rows : undefined,
        multiset: [...rows].sort(),
      };
    }
    case "ask":
      return { kind: "ask", boolean: result.boolean };
    case "graph":
      return { kind: "graph", nquads: result.dataset.canonicalize() };
    default:
      throw new Error(`unexpected result kind ${result.kind}`);
  }
}

test("every sync-suite query answers identically through queryAsync, eight at a time, yielding at every poll", async () => {
  assert.ok(QUERIES.length >= 40, `${QUERIES.length} queries`);
  assert.equal(new Set(QUERIES.map((entry) => entry.name)).size, QUERIES.length, "distinct names");
  assert.equal(new Set(QUERIES.map((entry) => entry.query)).size, QUERIES.length, "distinct queries");

  const engine = new QueryEngine();
  const datasets = Object.fromEntries(
    Object.entries(DATASETS).map(([name, { syntax, text }]) => [name, Dataset.parse(text, syntax)]),
  );
  const idle = stackPointer();

  // The oracle, synchronously, before any job runs.
  const expected = QUERIES.map(({ data, query }) => canonical(engine.query(datasets[data], query), /ORDER BY/.test(query)));
  for (const [index, answer] of expected.entries()) {
    // A fixture whose oracle is empty cannot tell an honoured query from a dropped one.
    const nonEmpty =
      answer.kind === "ask" || (answer.kind === "select" ? answer.multiset.length > 0 : answer.nquads.length > 0);
    assert.ok(nonEmpty, `${QUERIES[index].name} has a non-empty oracle`);
  }

  // Eight workers draw from one queue; each job suspends at every poll. The jobs share
  // one engine of their own, so its plan cache is filled from inside interleaved jobs
  // rather than handed over warm by the oracle.
  const asyncEngine = new QueryEngine();
  const observed = new Array(QUERIES.length);
  const suspensions = [];
  let next = 0;
  let inFlight = 0;
  let peak = 0;
  const worker = async () => {
    while (next < QUERIES.length) {
      const index = next;
      next += 1;
      const { data, query } = QUERIES[index];
      inFlight += 1;
      peak = Math.max(peak, inFlight);
      try {
        const governed = await asyncEngine.queryGovernedAsync(datasets[data], query, { yieldEveryPolls: 0 });
        suspensions.push({ name: QUERIES[index].name, ...governed.evidence.async });
        assert.equal(governed.isComplete, true, QUERIES[index].name);
        const answer = await asyncEngine.queryAsync(datasets[data], query, { yieldEveryPolls: 0 });
        observed[index] = canonical(answer, /ORDER BY/.test(query));
      } finally {
        inFlight -= 1;
      }
    }
  };
  await Promise.all(Array.from({ length: CONCURRENCY }, worker));

  assert.equal(peak, CONCURRENCY, "eight jobs were in flight at once");
  // `yieldEveryPolls: 0` suspends at every poll. A DESCRIBE of a constant IRI evaluates
  // no algebra and so polls nothing; every other query polls, and so interleaves.
  for (const { name, polls, yields } of suspensions) {
    assert.equal(yields, polls, `${name} yielded at every poll`);
  }
  const interleaved = suspensions.filter(({ yields }) => yields > 0).map(({ name }) => name);
  assert.deepEqual(
    suspensions.filter(({ yields }) => yields === 0).map(({ name }) => name).sort(),
    ["describe-graph-star", "describe-person"],
  );
  assert.ok(interleaved.length >= 40, `${interleaved.length} jobs suspended mid-evaluation`);
  for (const [index, entry] of QUERIES.entries()) {
    assert.deepEqual(observed[index], expected[index], entry.name);
  }
  assert.equal(stackPointer(), idle, "the stack pointer is back at its idle value");
});
