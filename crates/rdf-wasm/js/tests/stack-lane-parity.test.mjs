// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: This checks the shipped JavaScript package's synchronous and asynchronous WebAssembly entries and error properties.

import { test } from "node:test";
import assert from "node:assert/strict";

import { Dataset, QueryEngine, ready } from "../index.mjs";

await ready();

const EX = "http://example.org/";
const TRIPLE = `?s <${EX}p> ?o . `;
const DATA = Dataset.parse(
  `<${EX}s> <${EX}p> <${EX}o> .\n<${EX}s> <${EX}p> <${EX}o> <${EX}g> .\n`,
  "nquads",
);
const engine = new QueryEngine();
const nested = (open, depth) =>
  `SELECT ?s WHERE { ${TRIPLE}${open.repeat(depth)}${TRIPLE}${"}".repeat(depth)} }`;
const shapes = [
  ["OPTIONAL", (depth) => nested(`OPTIONAL { ${TRIPLE}`, depth), () => 1],
  ["EXISTS", (depth) => nested(`FILTER EXISTS { ${TRIPLE}`, depth), () => 1],
  ["GRAPH", (depth) => `SELECT ?s WHERE { ${`GRAPH <${EX}g> { `.repeat(depth)}${TRIPLE}${"}".repeat(depth)} }`, () => 1],
  ["MINUS", (depth) => nested(`MINUS { ${TRIPLE}`, depth), (depth) => depth % 2 ? 0 : 1],
  ["LATERAL", (depth) => nested(`LATERAL { ${TRIPLE}`, depth), () => 1],
  ["SERVICE", (depth) => nested(`SERVICE SILENT <${EX}sparql> { ${TRIPLE}`, depth), () => 1],
];

async function outcome(query, lane) {
  try {
    const result = lane === "sync" ? engine.select(DATA, query) : await engine.selectAsync(DATA, query);
    return { rows: result.rowCount };
  } catch (error) {
    assert.match(error.code, /^native-sparql-(?:evaluation|host)-stack-exhausted$/);
    return { code: error.code, message: error.message };
  }
}

async function endpoint(shape, lane, expectedRows) {
  let answer = 1;
  let refusal = 512;
  assert.deepEqual(await outcome(shape(answer), lane), { rows: expectedRows(answer) });
  assert.ok((await outcome(shape(refusal), lane)).code);
  while (refusal - answer > 1) {
    const middle = (answer + refusal) >> 1;
    const result = await outcome(shape(middle), lane);
    if ("rows" in result) answer = middle;
    else refusal = middle;
  }
  assert.deepEqual(await outcome(shape(answer), lane), { rows: expectedRows(answer) });
  return { answer, refusal, error: await outcome(shape(refusal), lane) };
}

test("six nested shapes have identical answer endpoints and refusal wording on both package lanes", async (t) => {
  for (const [name, shape, expectedRows] of shapes) {
    const sync = await endpoint(shape, "sync", expectedRows);
    const asyncLane = await endpoint(shape, "async", expectedRows);
    assert.deepEqual(asyncLane, sync, `${name}: both lanes answer and refuse at the same depth with the same error`);
    assert.deepEqual(await outcome(shape(1), "sync"), { rows: expectedRows(1) }, `${name}: the instance still answers synchronously`);
    assert.deepEqual(await outcome(shape(1), "async"), { rows: expectedRows(1) }, `${name}: the instance still answers asynchronously`);
    t.diagnostic(`${name}: ${sync.answer} answers; ${sync.refusal} refuses with ${sync.error.code}`);
  }
});
