// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Node real-execution conformance for `aggregateNamespace` — the wasm surface for
// purrdf's first-party statistical aggregate set (`MEDIAN`, `PERCENTILE`, `STDDEV`,
// `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`, `FIRST`, `LAST`, `TOPK`,
// `purrdf_sparql_eval::stat_agg`), driven against the ACTUAL optimized wasm module.
//
// `AggregateRegistry::register_statistical_aggregates` takes only an IRI namespace
// string, so `queryGoverned`/`updateGoverned`'s `aggregateNamespace` option is the
// WHOLE surface: no callback, no per-aggregate marshaling. This file drives a real
// `MEDIAN` query and a real UPDATE whose WHERE clause folds one through a nested
// `SELECT … GROUP BY`, and asserts the COMPUTED values — not merely that the option
// parses.

import { test } from "node:test";
import assert from "node:assert/strict";

import { Dataset, QueryEngine, ready } from "../index.mjs";

await ready();

const NS = "https://example.org/agg#";

const TRIG = `
@prefix ex: <https://example.org/> .
ex:a ex:value 1 .
ex:b ex:value 2 .
ex:c ex:value 3 .
`;

const MEDIAN_QUERY = `PREFIX ex: <https://example.org/>
SELECT (AGG(<${NS}MEDIAN>, ?v) AS ?m) WHERE { ?s ex:value ?v }`;

test("queryGoverned computes MEDIAN through aggregateNamespace", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");

  const outcome = engine.queryGoverned(ds, MEDIAN_QUERY, { aggregateNamespace: NS });
  assert.equal(outcome.isComplete, true);
  assert.equal(outcome.tripped, undefined);
  const rows = outcome.result.rows.toArray();
  assert.equal(rows.length, 1);
  assert.equal(rows[0].m.value, "2");
});

test("omitting aggregateNamespace leaves the ten names unregistered, with the existing typed error", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");

  assert.throws(() => engine.queryGoverned(ds, MEDIAN_QUERY), (error) => {
    return /custom.aggregate|not registered|unregistered|native-sparql-aggregate-function/i.test(
      error.message,
    );
  });
});

test("updateGoverned reaches MEDIAN from a DELETE/INSERT WHERE nested SELECT", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");

  const update = `PREFIX ex: <https://example.org/>
    INSERT { ex:summary ex:median ?m }
    WHERE {
      SELECT (AGG(<${NS}MEDIAN>, ?v) AS ?m) WHERE { ?s ex:value ?v }
    }`;

  const outcome = engine.updateGoverned(ds, update, { aggregateNamespace: NS });
  assert.equal(outcome.isApplied, true);

  const check = engine.select(
    ds,
    "PREFIX ex: <https://example.org/> SELECT ?m WHERE { ex:summary ex:median ?m }",
  );
  const rows = check.rows.toArray();
  assert.equal(rows.length, 1);
  assert.equal(rows[0].m.value, "2");
});

test("an ungoverned entry refuses aggregateNamespace rather than ignoring it", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  assert.throws(
    () => engine.select(ds, MEDIAN_QUERY, { aggregateNamespace: NS }),
    TypeError,
  );
  assert.throws(
    () => engine.query(ds, MEDIAN_QUERY, { aggregateNamespace: NS }),
    TypeError,
  );
});

test("queryEntailmentGoverned refuses aggregateNamespace rather than ignoring it", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  assert.throws(
    () =>
      engine.queryEntailmentGoverned(ds, MEDIAN_QUERY, "rdfs", {
        aggregateNamespace: NS,
      }),
    TypeError,
  );
});
