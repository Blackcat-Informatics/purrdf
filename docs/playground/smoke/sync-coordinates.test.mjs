// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: the assembled JavaScript Worker message handler must forward exact
// strings and structured-cloned BigInts through the shipped synchronous package.
import { test } from "node:test";
import assert from "node:assert/strict";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

test("playground synchronous query messages forward exact coordinates and errors", async () => {
  const output = process.env.PLAYGROUND_OUT;
  assert.ok(output, "PLAYGROUND_OUT must point at the unchanged Worker and actual package assembly");
  let handle;
  let response;
  globalThis.self = {
    location: { origin: "https://playground.example" },
    addEventListener(type, listener) { if (type === "message") handle = listener; },
    postMessage(message) { response = message; },
  };
  await import(pathToFileURL(join(output, "engine.worker.mjs")).href);
  const call = async (op, args) => {
    response = undefined;
    await handle({ origin: "", data: structuredClone({ id: op, op, args }) });
    assert.equal(response.id, op);
    return response;
  };
  const loaded = await call("parse", { text: '<https://example.org/s> <https://example.org/p> "present" .', format: "ntriples" });
  assert.equal(loaded.ok, true, loaded.error);
  const now = "-12345-06-07T08:09:10.123456789123456789+05:30";
  const query = { sparql: "SELECT (NOW() AS ?t) (RAND() AS ?r) WHERE {}", now, entropySeed: 18446744073709551615n };
  const first = await call("query", query);
  assert.equal(first.ok, true, first.error);
  assert.equal(first.result.kind, "srj");
  assert.equal(first.result.srj.results.bindings[0].t.value, now);
  assert.deepEqual((await call("query", query)).result, first.result);
  const refused = await call("query", { ...query, entropySeed: 18446744073709551616n });
  assert.equal(refused.ok, false);
  assert.match(refused.error, /entropySeed/);
  const unchanged = await call("quads", {});
  assert.equal(unchanged.ok, true, unchanged.error);
  assert.equal(unchanged.result.size, 1);
});
