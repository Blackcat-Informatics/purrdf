// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: Frozen native proof bytes must cross real wasm-bindgen exports,
// JavaScript cancellation handles, BigInt counters and retained exception owners.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { ready, Reasoner, CancellationToken } from "../index.mjs";

await ready();
const vector = JSON.parse(await readFile(new URL("../../../validate/tests/fixtures/positive-host.json", import.meta.url), "utf8"));

test("positive world proofs preserve frozen native bytes and retained cause handles", () => {
  const retained = [];
  const receipts = [];
  const captureReceipts = (error) => {
    assert.ok(Array.isArray(JSON.parse(error.canonicalizationJson)));
    receipts.push([error, error.canonicalizationJson]);
  };
  const capture = (error, canonical = true) => {
    assert.ok(error instanceof Error);
    assert.equal(error instanceof WebAssembly.RuntimeError, false);
    assert.ok(JSON.parse(error.nativeCauseJson).code);
    retained.push([error.nativeCause, error.nativeCauseJson]);
    if (canonical) captureReceipts(error);
    return true;
  };
  const token = new CancellationToken();
  const cancellation = token.share();
  const producer = Reasoner.withPositiveProofs(vector.initial, vector.limits, 0, 0, "unlimited", token.share());
  token.free();
  const consumer = Reasoner.withPositiveProofs(vector.latest, vector.limits, 0, 0, "unlimited");
  try {
    assert.ok(Array.isArray(JSON.parse(producer.canonicalizationJson)));
    const query = producer.classSatisfiability("<urn:query>", "unlimited");
    query.free();
    const work = producer.advance(vector.latest, "unlimited");
    try {
      assert.equal(typeof work.scannedRows, "bigint");
      assert.equal(typeof work.newRows, "bigint");
      assert.ok(Array.isArray(JSON.parse(work.canonicalizationJson)));
    } finally {
      work.free();
    }
    const answer = producer.entails(vector.axiom, "unlimited");
    try {
      assert.equal(answer.answer, vector.answer);
      assert.equal(answer.certificate, vector.certificate);
      assert.equal(answer.proof, vector.proof);
      assert.throws(() => consumer.checkPositiveProof("entails", vector.axiom, answer.answer, answer.certificate, "malformed", "unlimited"), capture);
      const checked = consumer.checkPositiveProof("entails", vector.axiom, answer.answer, answer.certificate, answer.proof, "unlimited");
      try {
        assert.equal(checked.value, vector.report);
        assert.ok(Array.isArray(JSON.parse(checked.canonicalizationJson)));
      } finally {
        checked.free();
      }
    } finally {
      answer.free();
    }
    cancellation.cancel();
    assert.throws(() => producer.advance(vector.latest, "unlimited"), capture);
    assert.throws(() => consumer.advance(vector.latest, "invalid policy"), (error) => {
      assert.ok(error instanceof Error);
      assert.equal(error.canonicalizationJson, "[]");
      captureReceipts(error);
      return true;
    });
    assert.throws(() => consumer.work(), (error) => {
      capture(error, false);
      assert.equal(JSON.parse(error.nativeCauseJson).fields.cause.value.code, "reasoning-positive-consumed");
      return true;
    });
  } finally {
    producer.free();
    consumer.free();
    cancellation.free();
  }
  for (const [error, json] of receipts) {
    assert.equal(error.canonicalizationJson, json);
  }
  for (const [cause, json] of retained) {
    try {
      assert.equal(cause.json, json);
    } finally {
      cause.free();
    }
  }
});
