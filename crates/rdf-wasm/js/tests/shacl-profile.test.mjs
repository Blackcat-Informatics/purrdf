// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: drives the shaclProfile argument of the built npm package root's shaclValidateToSarif and its async twin through real JavaScript calls

// The dated SHACL law `shaclValidateToSarif` selects with `shaclProfile`, reached through the
// PUBLIC package root. A parameter that was read but never threaded through would be
// invisible: the call still succeeds and the compatibility law answers instead. So the law is
// pinned by two vendored SHACL 1.2 suite entries whose answers separate the laws, each
// refusal beside its valid neighbour:
//
// * sparql/pre-binding/pre-binding-009: the compatibility law refuses the shapes graph at
//   load; the editor's-draft law (shacl-ed-2026-10-16) validates it.
// * sparql/node/prefixes-003: the working draft (shacl-wd-2026-09-18) refuses the shapes
//   graph at load; the editor's-draft law validates it.
// * shacl-ed-2026-10-17 names no law: the refusal carries the unsupported-profile status.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { ready, shaclValidateToSarif, shaclValidateToSarifAsync } from "../index.mjs";

await ready();

const SUITE = new URL("../../../../vectors/shacl12/tests/", import.meta.url);
const suite = (relative) => readFileSync(fileURLToPath(new URL(relative, SUITE)), "utf8");
const BASE = "https://example.org/suite/entry.ttl";
const ED = "shacl-ed-2026-10-16";
const WD = "shacl-wd-2026-09-18";
const REC = "shacl-rec-2017-07-20";
const XPATH_20 = "xpath-2.0-2010-12-14";
const XPATH_31 = "xpath-3.1-2017-03-21";

const PRE_BINDING_009 = suite("sparql/pre-binding/pre-binding-009.ttl");
const PRE_BINDING_009_DATA =
  '<http://example.com/ns#InvalidResource> <http://example.com/ns#property> "forbidden" .\n' +
  '<http://example.com/ns#ValidResource1> <http://example.com/ns#property> "fine" .\n';
const PREFIXES_003 = suite("sparql/node/prefixes-003.ttl");
const PREFIXES_003_DATA =
  "<http://example.com/ns#InvalidResource1> <http://example.com/ns#property> <http://test.com/ns#Value> .\n";

const validate = (shapes, data, xpathRegex, shaclProfile) =>
  shaclValidateToSarif(shapes, data, BASE, undefined, undefined, undefined, undefined, undefined, xpathRegex, shaclProfile);

test("pre-binding-009 is refused at load by the default law and validated by the editor's draft", () => {
  assert.throws(
    () => validate(PRE_BINDING_009, PRE_BINDING_009_DATA),
    /VALUES is not allowed/,
  );
  const log = JSON.parse(validate(PRE_BINDING_009, PRE_BINDING_009_DATA, undefined, ED));
  assert.equal(log.runs[0].results.length, 1);
  assert.match(JSON.stringify(log.runs[0].results), /InvalidResource/);
});

test("prefixes-003 is refused at load by the working draft and validated by the editor's draft", () => {
  assert.throws(
    () => validate(PREFIXES_003, PREFIXES_003_DATA, undefined, WD),
    /undeclared prefix/,
  );
  const log = JSON.parse(validate(PREFIXES_003, PREFIXES_003_DATA, undefined, ED));
  assert.equal(log.runs[0].results.length, 1);
});

test("an unknown SHACL profile id is the unsupported-profile status", () => {
  for (const unknown of ["shacl-ed-2026-10-17", "SHACL-ED-2026-10-16", ""]) {
    assert.throws(
      () => validate(PREFIXES_003, PREFIXES_003_DATA, undefined, unknown),
      /unsupported-profile/,
      JSON.stringify(unknown),
    );
  }
  // The exact id beside the refused ones is accepted.
  assert.doesNotThrow(() => validate(PREFIXES_003, PREFIXES_003_DATA, undefined, ED));
});

test("a dated law refuses an XPath override of another law and accepts its own", () => {
  assert.throws(
    () => validate(PREFIXES_003, PREFIXES_003_DATA, XPATH_20, ED),
    /requires/,
  );
  assert.doesNotThrow(() => validate(PREFIXES_003, PREFIXES_003_DATA, XPATH_31, ED));
  // The 2017 Recommendation requires XPath 2.0: the pair the other way round.
  assert.throws(
    () => validate(PREFIXES_003, PREFIXES_003_DATA, XPATH_31, REC),
    /requires/,
  );
  // The compatibility law leaves the pattern law to the caller.
  assert.doesNotThrow(() => validate(PREFIXES_003, PREFIXES_003_DATA, XPATH_20));
});

test("the asynchronous twin takes the law in its options object", async () => {
  const twin = (shapes, data, options) =>
    shaclValidateToSarifAsync(shapes, data, BASE, null, null, null, null, null, options);
  await assert.rejects(
    twin(PRE_BINDING_009, PRE_BINDING_009_DATA, null),
    /VALUES is not allowed/,
  );
  const async = await twin(PRE_BINDING_009, PRE_BINDING_009_DATA, { shaclProfile: ED });
  assert.equal(
    async,
    validate(PRE_BINDING_009, PRE_BINDING_009_DATA, undefined, ED),
    "the twin answers byte for byte as the synchronous entry",
  );
  await assert.rejects(
    twin(PREFIXES_003, PREFIXES_003_DATA, { shaclProfile: WD }),
    /undeclared prefix/,
  );
  await assert.rejects(
    twin(PREFIXES_003, PREFIXES_003_DATA, { shaclProfile: "shacl-ed-2026-10-17" }),
    /unsupported-profile/,
  );
});
