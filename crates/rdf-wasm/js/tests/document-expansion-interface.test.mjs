// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: This smoke must call the shipped JavaScript and wasm-bindgen exports in Node to verify actual byte transport and native error ownership.
import { test } from "node:test";
import assert from "node:assert/strict";
import { expandDocument, compactDocument, flattenDocument, frameDocument, ready, YamlSourceCause } from "../index.mjs";
await ready();
test("document operation exports preserve byte transport and native error ownership", () => {
  const bytes = text => new TextEncoder().encode(text), source = bytes('{"@id":"urn:s","urn:p":"javascript"}');
  for (const [call, frames, request, expected, code] of [
    [expandDocument, [], {version:1,format:"jsonld"}, '[{"@id":"urn:s","urn:p":[{"@value":"javascript"}]}]', "document-expansion"],
    [compactDocument, [], {version:1,format:"jsonld",output_context:{}}, '{"@context":{},"@id":"urn:s","urn:p":"javascript"}', "document-compaction"],
    [flattenDocument, [], {version:1,format:"jsonld",output_context:{}}, '{"@context":{},"@id":"urn:s","urn:p":"javascript"}', "document-flattening"],
    [frameDocument, [bytes('{}')], {version:1,format:"jsonld"}, '{"@context":{},"@id":"urn:s","urn:p":"javascript"}', "document-framing"],
  ]) {
    assert.equal(call(source, ...frames, bytes(JSON.stringify(request))), expected);
    assert.throws(() => call(frames.length ? source : null, ...frames.map(() => null), bytes('{"version":2,"format":"jsonld"}')), error => {
      assert.equal(error.code, `${code}-request`);
      error.nativeCause?.free(); return true;
    });
    assert.throws(() => call(source, ...frames, bytes(JSON.stringify({...request,output:{max_bytes:0}}))), error => {
      assert.deepEqual([error.code, error.diagnostic.code], [`${code}-output`, `${code}-output`]);
      assert.deepEqual([error.nativeCause instanceof YamlSourceCause, error.nativeCause.json], [true, error.nativeCauseJson]);
      assert.match(error.nativeCauseJson, /json-output-bytes/);
      error.nativeCause.free(); return true;
    });
  }
});
