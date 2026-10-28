// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: these cases verify JavaScript positional ABI, checked host values,
// owned wasm-bindgen requests and actual JSPI suspension through the shipped package.
import { test } from "node:test";
import assert from "node:assert/strict";
import { ready, shaclEntail, shaclApplyRules, shaclEntailAsync, shaclApplyRulesAsync } from "../index.mjs";
import * as raw from "../pkg/purrdf_wasm.js";
import { runJob } from "../pkg/purrdf_jspi.mjs";
await ready();
const NOW = "-12345-06-07T08:09:10.123456789123456789123456789+05:30";
const SHAPES = `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:S a sh:NodeShape ; sh:targetNode ex:s ; sh:rule ex:r .
ex:r a sh:SPARQLRule ; sh:runOnce true ; sh:construct """
 PREFIX ex: <http://example.org/>
 CONSTRUCT { ex:s ex:time ?time ; ex:uuid ?uuid ; ex:text ?text }
 WHERE { BIND(NOW() AS ?time) BIND(UUID() AS ?uuid)
 BIND(STRUUID() AS ?text) FILTER(REGEX("ok", "^ok$")) }
""" .`;
const SRL = `PREFIX : <http://example.org/>
RULE { :s :time ?time ; :uuid ?uuid ; :text ?text }
WHERE { SET(?time := NOW()) SET(?uuid := UUID()) SET(?text := STRUUID())
 FILTER(REGEX("ok", "^ok$")) }`;
const entail = [SHAPES, "", undefined, undefined, undefined, undefined, 8n, 32n, undefined, undefined];
const apply = (shapes, srl) => ["", shapes, srl, undefined, undefined, true, undefined, undefined, undefined, 8n, 32n, undefined, undefined];
const routes = [
  [shaclEntail, raw.shaclEntail, shaclEntailAsync, "entail", "takeShaclEntailment", entail],
  [shaclApplyRules, raw.shaclApplyRules, shaclApplyRulesAsync, "applyRules", "takeShaclRulesInference", apply(SHAPES)],
  [shaclApplyRules, raw.shaclApplyRules, shaclApplyRulesAsync, "applyRules", "takeShaclRulesInference", apply(undefined, SRL)],
];
function value(result) {
  try { const receipt = JSON.parse(result.executionReceiptJson); assert.ok(Array.isArray(receipt.executions)); return { text: result.ntriples ?? result.inferred, diagnostics: result.diagnostics, proof: result.proof }; }
  finally { result.free(); }
}
function begin(route, controls) {
  const options = raw.AsyncJobOptions.fromJs(raw.AsyncOperationKind.Shacl, controls, undefined, false, undefined);
  try { return raw.AsyncJob.beginShacl(options, raw.ShaclJobRequest[route[3]](...route[5])); }
  finally { options.free(); }
}
async function low(route, controls) {
  const job = begin(route, controls);
  try { assert.equal(await runJob(job, {}), raw.RunStatus.Outcome); return value(job[route[4]]()); }
  finally { job.finish(); job.free(); }
}

test("rule coordinates cross synchronous async and low-level positional routes", async () => {
  assert.equal(raw.ShaclJobRequest.entail.length, 10);
  assert.equal(raw.ShaclJobRequest.applyRules.length, 13);
  for (const route of routes) for (const xpathRegex of [undefined, "xpath-2.0-2010-12-14", "xpath-3.1-2017-03-21"]) {
    for (const entropySeed of [0n, 18446744073709551615n]) {
      const controls = { now: NOW, entropySeed, xpathRegex };
      const expected = value(route[0](...route[5], xpathRegex, NOW, entropySeed));
      assert.ok(expected.text.includes(NOW));
      assert.deepEqual(value(route[1](...route[5], xpathRegex, NOW, entropySeed)), expected);
      assert.deepEqual(value(await route[2](...route[5], controls)), expected);
      assert.deepEqual(await low(route, controls), expected);
    }
  }
});

test("rule host coordinate refusals precede document parsing and preserve reuse", async () => {
  for (const route of routes) {
    const invalid = [...route[5]];
    invalid[0] = "invalid document";
    for (const controls of [{ now: 7 }, { now: "not a date" }, { entropySeed: -1n }, { entropySeed: 18446744073709551616n }, { entropySeed: Number.MAX_SAFE_INTEGER + 1 }, { entropySeed: "7" }]) {
      assert.throws(() => route[0](...invalid, undefined, controls.now, controls.entropySeed), /now|entropySeed/);
      await assert.rejects(route[2](...invalid, controls), /now|entropySeed/);
    }
    assert.ok(value(await route[2](...route[5], { now: NOW, entropySeed: 0 })).text.includes(NOW));
  }
  for (const request of [
    () => raw.ShaclJobRequest.validateToSarif("", ""),
    () => raw.ShaclJobRequest.evalNodeExpr("", "", undefined, "<http://example.org/s>"),
    () => raw.ShaclJobRequest.productValidateToSarif(new Uint8Array(), ""),
  ]) for (const controls of [{ now: NOW }, { entropySeed: 0n }]) {
    const options = raw.AsyncJobOptions.fromJs(raw.AsyncOperationKind.Shacl, controls, undefined, false, undefined);
    try { assert.throws(() => raw.AsyncJob.beginShacl(options, request()), /only to entailment and rule execution/); }
    finally { options.free(); }
  }
});

test("rule coordinate owners survive actual JSPI yields and interleaving", async () => {
  for (const route of routes) {
    const controls = { now: NOW, entropySeed: 0n, yieldEveryPolls: 0 };
    const expected = value(route[0](...route[5], undefined, NOW, 0n));
    const job = begin(route, controls);
    controls.now = "2000-01-01T00:00:00Z";
    controls.entropySeed = 99n;
    let settled = false;
    const pending = runJob(job, {}).finally(() => { settled = true; });
    try {
      await new Promise((resolve) => setTimeout(resolve, 0));
      assert.equal(settled, false, "the first job is suspended when the other job starts");
      const other = value(await route[2](...route[5], controls));
      assert.ok(other.text.includes(controls.now));
      assert.notEqual(other.text, expected.text);
      assert.equal(await pending, raw.RunStatus.Outcome);
      const evidence = job.takeEvidence();
      try { assert.ok(evidence.yields > 0, "the owned request actually suspended through JSPI"); }
      finally { evidence.free(); }
      assert.deepEqual(value(job[route[4]]()), expected);
    } finally { await pending; job.finish(); job.free(); }
  }
});

test("rule omitted coordinates retain independent host defaults", async () => {
  const random = Math.random;
  const dateNow = Date.now;
  let draws = 0;
  Math.random = () => { draws += 1; return 0.25; };
  Date.now = () => 946684800000;
  try {
    for (const route of routes) for (const asynchronous of [false, true]) {
      const run = async (now, entropySeed) => value(asynchronous
        ? await route[2](...route[5], { now, entropySeed })
        : route[0](...route[5], undefined, now, entropySeed));
      draws = 0;
      assert.ok((await run(NOW, 0n)).text.includes(NOW));
      assert.equal(draws, 0);
      assert.ok((await run(NOW, null)).text.includes(NOW));
      assert.ok(draws > 0);
      draws = 0;
      assert.ok((await run(null, 0n)).text.includes("2000-01-01"));
      assert.equal(draws, 0);
      await run(undefined, undefined);
      assert.ok(draws > 0);
    }
  } finally { Math.random = random; Date.now = dateNow; }
});

test("rule supplied coordinates preserve signal cancellation and subsequent reuse", async () => {
  for (const route of routes) {
    const controller = new AbortController();
    const reason = new Error("stop rule execution");
    const timer = setTimeout(() => controller.abort(reason), 0);
    try {
      await assert.rejects(route[2](...route[5], { now: NOW, entropySeed: 0n, signal: controller.signal, yieldEveryPolls: 0 }), (error) => error.cause === reason);
    } finally { clearTimeout(timer); }
    assert.ok(value(await route[2](...route[5], { now: NOW, entropySeed: 0n })).text.includes(NOW));
  }
});
