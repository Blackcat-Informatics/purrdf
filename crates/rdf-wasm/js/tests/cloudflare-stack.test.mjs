// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: This exercises the shipped Cloudflare JavaScript fetch adapter and its real optimized WebAssembly problem responses.

import { test } from "node:test";
import assert from "node:assert/strict";
import { Dataset, QueryEngine, ready } from "../index.mjs";
import { handleSparqlRequest } from "../cloudflare.mjs";

await ready();

const EX = "http://example.org/";
const TRIPLE = `?s <${EX}p> ?o . `;
const dataset = Dataset.parse(
  `<${EX}s> <${EX}p> <${EX}o> .\n<${EX}s> <${EX}p> <${EX}o> <${EX}g> .\n`,
  "nquads",
);
const options = { engine: new QueryEngine(), dataset, governors: { deadlineMs: 10_000 } };
const nested = (open, depth) =>
  `SELECT ?s WHERE { ${TRIPLE}${open.repeat(depth)}${TRIPLE}${"}".repeat(depth)} }`;
const shapes = [
  ["OPTIONAL", (depth) => nested(`OPTIONAL { ${TRIPLE}`, depth), () => 1],
  ["EXISTS", (depth) => nested(`FILTER EXISTS { ${TRIPLE}`, depth), () => 1],
  ["GRAPH", (depth) => `SELECT ?s WHERE { ${`GRAPH <${EX}g> { `.repeat(depth)}${TRIPLE}${"}".repeat(depth)} }`, () => 1],
  ["MINUS", (depth) => nested(`MINUS { ${TRIPLE}`, depth), (depth) => depth % 2 ? 0 : 1],
  ["LATERAL", (depth) => nested(`LATERAL { ${TRIPLE}`, depth), () => 1],
  ["SERVICE SILENT", (depth) => nested(`SERVICE SILENT <${EX}sparql> { ${TRIPLE}`, depth), () => 1],
];

const fetchQuery = (query, selected = options) =>
  handleSparqlRequest(
    new Request(`https://endpoint.example.org/sparql?query=${encodeURIComponent(query)}`),
    selected,
  );

async function outcome(query, expectedRows) {
  const response = await fetchQuery(query);
  if (response.status === 200) {
    const body = await response.json();
    assert.equal(body.results.bindings.length, expectedRows);
    return { status: 200 };
  }
  assert.equal(response.status, 400, "a nested request receives a typed client refusal");
  assert.equal(response.headers.get("Content-Type"), "application/problem+json");
  const body = await response.json();
  assert.match(body.code, /^native-sparql-(?:evaluation|host)-stack-exhausted$/);
  assert.equal(body.status, 400);
  assert.equal(body.title, "Bad Request");
  assert.match(body.detail, new RegExp(`^error ${body.code}: `));
  assert.equal(body.correlationId, undefined);
  return { status: 400, code: body.code, detail: body.detail };
}

test("Cloudflare fetch maps both stack guards to 400 at every nested shape's first refusal", async (t) => {
  for (const [name, shape, rows] of shapes) {
    let answer = 1;
    let refusal = 512;
    assert.deepEqual(await outcome(shape(answer), rows(answer)), { status: 200 }, `${name}: shallow answer`);
    assert.equal((await outcome(shape(refusal), rows(refusal))).status, 400, `${name}: deep refusal`);
    while (refusal - answer > 1) {
      const middle = (answer + refusal) >> 1;
      if ((await outcome(shape(middle), rows(middle))).status === 200) answer = middle;
      else refusal = middle;
    }
    assert.deepEqual(await outcome(shape(answer), rows(answer)), { status: 200 }, `${name}: last answer`);
    const rejected = await outcome(shape(refusal), rows(refusal));
    assert.equal(refusal, answer + 1);
    assert.equal((await outcome(shape(1), rows(1))).status, 200, `${name}: endpoint remains usable`);
    t.diagnostic(`${name}: ${answer} answers; ${refusal} refuses with ${rejected.code}`);
  }
});

test("Cloudflare fetch still answers a genuine XPath evaluation resource failure as 500", async () => {
  const xpath = { ...options, xpathRegex: "xpath-3.1-2017-03-21" };
  const pattern = "a".repeat((64 << 10) + 1);
  const query = `SELECT ?r WHERE { ?s <${EX}p> ?v BIND(REGEX(STR(?v), ${JSON.stringify(pattern)}) AS ?r) }`;
  const failure = await fetchQuery(query, xpath);
  assert.equal(failure.status, 500);
  const body = await failure.json();
  assert.equal(body.code, "xpath-pattern-bytes");
  assert.match(body.detail, /pattern.*bytes|source bytes/);
  assert.equal(body.correlationId, undefined);
  const healthy = await fetchQuery(`SELECT ?s WHERE { ?s <${EX}p> ?o }`, xpath);
  assert.equal(healthy.status, 200);
});
