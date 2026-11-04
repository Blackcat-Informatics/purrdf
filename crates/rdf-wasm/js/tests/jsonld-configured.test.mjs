// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
import { test } from "node:test";
import assert from "node:assert/strict";
import { CompiledJsonLdContext, Dataset, QueryEngine, ready } from "../index.mjs";
await ready();
const OPTIONS = JSON.stringify({version: 1, mode: "context", prefixes: {
  ex: "https://example.org/", schema: "https://schema.org/",
}});
const SOURCE = '<https://example.org/alice> <https://schema.org/name> "Alice" .\n';
test("configured Dataset serialization reuses compiled contexts", () => {
  const dataset = Dataset.parse(SOURCE, "nquads");
  const direct = dataset.serializeConfigured("jsonld", OPTIONS);
  const context = new CompiledJsonLdContext(OPTIONS);
  const reused = dataset.serializeWithContext("jsonld", context);
  assert.equal(direct, reused);
  assert.equal(JSON.parse(direct)["@graph"][0]["@id"], "ex:alice");
  assert.equal(JSON.parse(direct)["@graph"][0]["schema:name"]["@value"], "Alice");
});
test("configured graph results and YAML schema headers use the same engine", () => {
  const dataset = Dataset.parse(SOURCE, "nquads");
  const engine = new QueryEngine();
  const query = "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }";
  const jsonld = engine.queryRawConfigured(dataset, query, undefined, "jsonld", OPTIONS).body;
  assert.equal(JSON.parse(jsonld)["@graph"][0]["@id"], "ex:alice");
  const context = new CompiledJsonLdContext(OPTIONS);
  const yaml = engine.queryRawWithContext(dataset, query, undefined, "yamlld", context,
    "https://example.org/purrdf.schema.json").body;
  assert.match(yaml, /^# yaml-language-server: \$schema=https:\/\/example\.org\/purrdf\.schema\.json/m);
});
test("closed options fail before output", () => {
  const dataset = Dataset.parse(SOURCE, "nquads");
  assert.throws(() => dataset.serializeConfigured("jsonld", '{"version":1,"mode":"expanded","extra":true}'), /unknown/);
});
test("configured YAML input crosses wasm-bindgen with native error records", () => {
  const source = "'@id': first\nhttps://example.org/p: one\n---\n'@id': second\nhttps://example.org/p: two\n";
  const options = '{"version":1,"yaml_ld":{"extract_all_scripts":true}}';
  const first = Dataset.parse(source, "yamlld", "https://example.org/");
  const all = Dataset.parse(source, "yamlld", "https://example.org/", false, options);
  assert.equal(first.size, 1); assert.equal(all.size, 2);
  assert.equal(all.rememberEmptyGraphs, false);
  assert.ok(all.serialize("nquads").includes("<https://example.org/second>"));
  for (const [input, format, config, code] of [
    [source, "yamlld", '{"version":1,"yaml_ld":{"syntax":{"max_input_bytes":0}}}', "native-yaml-input-limit"],
    ["42: value\n", "yamlld", options, "mapping-key-error"],
    ["", "turtle", options, "yamlld-input-options-format"],
  ]) {
    assert.throws(() => Dataset.parse(input, format, undefined, undefined, config), error => {
      assert.ok(error instanceof Error); assert.equal(error.code, code);
      assert.equal(error.diagnostic.code, code);
      if (format === "yamlld") assert.deepEqual(error.diagnostic.location, {line: "1", column: 1});
      return true;
    });
  }
  first.free(); all.free();
});
