// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: this exercises intrinsic JavaScript buffer imports, wasm-bindgen
// ownership transfers and caught host/native exceptions in the shipped package.

import { test } from "node:test";
import assert from "node:assert/strict";
import { ready, yamlSourceExport, yamlSourceReconstruct } from "../index.mjs";

await ready();

test("YAML source pair preserves owned bytes and rejects before bridge copy", () => {
  const encoder = new TextEncoder();
  const config = {
    version: 1, name: "host-yaml-source", profile_version: 1,
    vocabulary: "https://example.org/yaml#",
  };
  const profile = encoder.encode(JSON.stringify(config));
  const expected = encoder.encode("\uFEFF# original\r\na: [one, 'two']\r\n");
  const source = expected.slice();
  const exported = yamlSourceExport(source, "https://example.org/source.yaml", profile);
  const root = exported.root;
  const dataset = exported.takeDataset();
  assert.ok(dataset);
  assert.equal(exported.takeDataset(), undefined);
  exported.free();
  source.fill(0);
  assert.deepEqual(yamlSourceReconstruct(dataset, root, profile), expected);

  const limited = encoder.encode(JSON.stringify({
    ...config, bounds: { syntax: { max_input_bytes: 0 } },
  }));
  const shadowed = encoder.encode("x\n");
  Object.defineProperties(shadowed, {
    length: { get() { throw new Error("source.length must not be read"); } },
    byteLength: { get() { throw new Error("source.byteLength must not be read"); } },
  });
  assert.throws(() => yamlSourceExport(shadowed, "https://example.org/source.yaml", limited), error => {
    assert.ok(error instanceof Error);
    assert.equal(error instanceof WebAssembly.RuntimeError, false);
    const native = JSON.parse(error.nativeCauseJson);
    assert.equal(native.code, "yaml-source-limit");
    assert.equal(native.fields.limit.value, "0");
    error.nativeCause.free();
    return true;
  });
  assert.throws(() => yamlSourceExport(shadowed, "https://example.org/source.yaml", new Uint8Array(16385)), error => {
    assert.equal(JSON.parse(error.nativeCauseJson).fields.resource.value, "profile JSON bytes");
    error.nativeCause.free();
    return true;
  });

  let retained;
  let retainedJson;
  // Mutate the actual Dataset, so reconstruction must freeze its current content.
  const quads = dataset.quads();
  assert.equal(dataset.delete(quads[0]), dataset);
  for (const quad of quads) quad.free();
  assert.throws(() => yamlSourceReconstruct(dataset, root, profile), error => {
    assert.ok(error instanceof Error);
    assert.equal(error.code, "yaml-source-native");
    assert.equal(error.diagnostic.code, error.code);
    retained = error.nativeCause;
    retainedJson = error.nativeCauseJson;
    assert.equal(retained.json, retainedJson);
    return true;
  });
  dataset.free();
  assert.equal(retained.json, retainedJson);
  retained.free();

  assert.throws(() => yamlSourceExport(encoder.encode("a: [\n"), "https://example.org/source.yaml", profile), error => {
    assert.equal(error.code, "yaml-source-native");
    const syntax = JSON.parse(error.nativeCauseJson).fields.cause.value;
    assert.equal(syntax.code, "yaml-document");
    assert.equal(syntax.fields.line.kind, "unsigned");
    assert.equal(syntax.fields.offset.kind, "byte-count");
    error.nativeCause.free();
    return true;
  });
  assert.throws(() => yamlSourceExport(new Uint8Array(new SharedArrayBuffer(1)),
    "https://example.org/source.yaml", profile), /non-shared ArrayBuffer/);
  const reused = yamlSourceExport(expected, "https://example.org/source.yaml", profile);
  const restored = reused.takeDataset();
  const output = yamlSourceReconstruct(restored, reused.root, profile);
  reused.free(); restored.free();
  assert.ok(output instanceof Uint8Array);
  assert.deepEqual(output, expected);
});
