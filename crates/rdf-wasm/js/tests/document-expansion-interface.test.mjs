// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: This smoke must call the shipped JavaScript and wasm-bindgen exports in Node to verify actual byte transport and native error ownership.
import { test } from "node:test";
import assert from "node:assert/strict";
import { expandDocument, ready, YamlSourceCause } from "../index.mjs";
await ready();

test("document expansion exports preserve byte transport and native error ownership", () => {
  const bytes = text => new TextEncoder().encode(text);
  const source = bytes('{"urn:p":"javascript"}');
  const request = bytes('{"version":1,"format":"jsonld"}');
  assert.equal(expandDocument(source, request), '[{"urn:p":[{"@value":"javascript"}]}]');
  assert.throws(() => expandDocument(null, bytes('{"version":2,"format":"jsonld"}')), error => {
    assert.equal(error.code, "document-expansion-request");
    error.nativeCause?.free();
    return true;
  });
  assert.throws(() => expandDocument(source, bytes('{"version":1,"format":"jsonld","output":{"max_bytes":0}}')), error => {
    assert.equal(error.code, "document-expansion-output");
    assert.equal(error.diagnostic.code, error.code);
    assert.ok(error.nativeCause instanceof YamlSourceCause);
    assert.match(error.nativeCauseJson, /json-output-bytes/);
    assert.equal(error.nativeCause.json, error.nativeCauseJson);
    error.nativeCause.free();
    return true;
  });
});
