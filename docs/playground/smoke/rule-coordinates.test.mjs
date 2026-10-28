// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: the actual assembled Worker must carry structured-cloned BigInts
// and exact timestamps through its message interface without replacing its dataset.
import { test } from "node:test";
import assert from "node:assert/strict";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

test("playground rule messages forward coordinates and preserve dataset ownership", async () => {
  assert.ok(process.env.PLAYGROUND_OUT);
  let handle, response;
  globalThis.self = {
    location: { origin: "https://playground.example" },
    addEventListener(type, listener) { if (type === "message") handle = listener; },
    postMessage(message) { response = message; },
  };
  await import(pathToFileURL(join(process.env.PLAYGROUND_OUT, "engine.worker.mjs")).href);
  const call = async (op, args) => {
    response = undefined;
    await handle({ origin: "", data: structuredClone({ id: op, op, args }) });
    assert.equal(response.id, op);
    return response;
  };
  const loaded = await call("parse", { text: '<http://example.org/s> <http://example.org/p> "present" .', format: "ntriples" });
  assert.equal(loaded.ok, true, loaded.error);
  const before = (await call("quads", {})).result;
  const shapes = `@prefix sh: <http://www.w3.org/ns/shacl#> .
    <http://example.org/S> a sh:NodeShape ; sh:targetNode <http://example.org/s> ;
    sh:rule [ a sh:SPARQLRule ; sh:runOnce true ; sh:construct """
      CONSTRUCT { <http://example.org/s> <http://example.org/time> ?t ; <http://example.org/id> ?u }
      WHERE { BIND(NOW() AS ?t) BIND(UUID() AS ?u) } """ ] .`;
  const controls = { shapes, now: "-12345-06-07T08:09:10.123456789123456789+05:30", entropySeed: 18446744073709551615n };
  const first = await call("shaclEntail", controls);
  assert.equal(first.ok, true, first.error);
  assert.ok(first.result.ntriples.includes(controls.now));
  assert.deepEqual((await call("shaclEntail", controls)).result, first.result);
  const refused = await call("shaclEntail", { ...controls, entropySeed: 18446744073709551616n });
  assert.equal(refused.ok, false);
  assert.match(refused.error, /entropySeed/);
  const unsupported = await call("shaclValidate", controls);
  assert.equal(unsupported.ok, false);
  assert.match(unsupported.error, /now|entropySeed/);
  assert.deepEqual((await call("quads", {})).result, before);
});
