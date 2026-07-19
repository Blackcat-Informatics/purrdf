// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { test } from "node:test";
import assert from "node:assert/strict";

import {
  CompiledJsonLdContext,
  Dataset,
  QueryEngine,
  ready,
} from "../index.mjs";

await ready();

const OPTIONS = JSON.stringify({
  version: 1,
  mode: "context",
  prefixes: {
    ex: "https://example.org/",
    schema: "https://schema.org/",
  },
});
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
  const jsonld = engine.queryRawConfigured(dataset, query, undefined, "jsonld", OPTIONS);
  assert.equal(JSON.parse(jsonld)["@graph"][0]["@id"], "ex:alice");

  const context = new CompiledJsonLdContext(OPTIONS);
  const yaml = engine.queryRawWithContext(
    dataset,
    query,
    undefined,
    "yamlld",
    context,
    "https://example.org/purrdf.schema.json",
  );
  assert.match(yaml, /^# yaml-language-server: \$schema=https:\/\/example\.org\/purrdf\.schema\.json/m);
});

test("closed options fail before output", () => {
  const dataset = Dataset.parse(SOURCE, "nquads");
  assert.throws(
    () => dataset.serializeConfigured("jsonld", '{"version":1,"mode":"expanded","extra":true}'),
    /unknown/,
  );
});
