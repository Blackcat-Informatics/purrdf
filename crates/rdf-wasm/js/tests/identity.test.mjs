// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution conformance for the graph-IDENTITY surface reached through the
// PUBLIC package root — `Dataset.canonicalize()` (canonical line-sequence N-Quads) and
// `Dataset.isomorphic(other)` (graph isomorphism under blank-node relabeling), exactly
// as the docs playground's identity/diff pane calls them.

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, Dataset, QueryEngine } from "../index.mjs";
import { canonicalValue } from "./fixtures/poisoned-surface.mjs";
await ready();

test("dataset identities and mutation generations use exact bigint transport", () => {
  const dataset = new Dataset();
  const snapshot = dataset.snapshot();
  assert.equal(typeof dataset.id, "bigint");
  assert.equal(typeof dataset.generation, "bigint");
  assert.ok(dataset.id > 0n);
  assert.notEqual(dataset.id, snapshot.id);
  assert.equal(dataset.generation, 0n);
  const identity = dataset.id;
  new QueryEngine().update(dataset, "INSERT DATA { <https://example.org/s> <https://example.org/p> <https://example.org/o> }").dataset;
  assert.equal(dataset.id, identity);
  assert.equal(dataset.generation, 1n);
  assert.equal(snapshot.generation, 0n);
});

// The SAME graph with different blank-node labels + different statement order.
const A = `@prefix ex: <http://example.org/> .
_:a ex:name "Ann" ; ex:knows _:b .
_:b ex:name "Bob" .
`;
const B = `@prefix ex: <http://example.org/> .
_:x ex:name "Bob" .
_:y ex:name "Ann" ; ex:knows _:x .
`;
// A genuinely different graph (Bob knows a third party).
const C = `@prefix ex: <http://example.org/> .
_:a ex:name "Ann" ; ex:knows _:b .
_:b ex:name "Bob" ; ex:knows _:c .
_:c ex:name "Cid" .
`;

test("isomorphic() is true for the same graph under blank-node relabeling", () => {
  const a = Dataset.parse(A, "turtle");
  const b = Dataset.parse(B, "turtle");
  assert.equal(canonicalValue(a.isomorphic(b, "unlimited")), true);
});

test("isomorphic() is false for a structurally different graph", () => {
  const a = Dataset.parse(A, "turtle");
  const c = Dataset.parse(C, "turtle");
  assert.equal(canonicalValue(a.isomorphic(c, "unlimited")), false);
});

test("canonicalize() is stable and identifies isomorphic graphs byte-for-byte", () => {
  const a = Dataset.parse(A, "turtle");
  const b = Dataset.parse(B, "turtle");
  const canonA = canonicalValue(a.canonicalize("unlimited"));
  assert.ok(canonA.length > 0, "canonical form is non-empty");
  // Re-canonicalizing the same dataset is deterministic.
  assert.equal(canonicalValue(a.canonicalize("unlimited")), canonA);
  // Isomorphic graphs canonicalize to the identical string — the identity guarantee.
  assert.equal(canonicalValue(b.canonicalize("unlimited")), canonA);
});

// ── canonicalize() throws a JsError on a refusal, rather than aborting ──────────
//
// The 13-node circulant of quadratic residues, in both directions, cannot be
// labelled without a search, so the finite bound "0" trips the labelling's typed
// `limit` refusal, which crosses the boundary as a thrown `JsError`. The same input
// completes under "unlimited", and a two-node cycle needs no search, so the same
// bound "0" admits it: the refusal is the input's, not the bound's. Before this
// surface threw a `JsError` for a refusal, it aborted the wasm process instead.
const CIRCULANT = Array.from({ length: 13 }, (_, u) => Array.from({ length: 13 - u - 1 }, (_, k) => u + k + 1)
  .filter((v) => [1, 3, 4, 9, 10, 12].includes((v - u) % 13))
  .map((v) => `_:v${u} <http://example.org/p> _:v${v} .\n_:v${v} <http://example.org/p> _:v${u} .\n`).join("")).join("");

test("canonicalize() throws on the labelling limit, naming it, rather than aborting", () => {
  const refused = Dataset.parse(CIRCULANT, "turtle");
  assert.throws(() => canonicalValue(refused.canonicalize("0")), /labelling refused/);
});

test("canonicalize() still completes the neighbours of the refused input", () => {
  const cycle = Dataset.parse("_:a <http://example.org/p> _:b .\n_:b <http://example.org/p> _:a .\n", "turtle");
  assert.ok(canonicalValue(cycle.canonicalize("0")).startsWith("_:0 "));
  const canon = canonicalValue(Dataset.parse(CIRCULANT, "turtle").canonicalize("unlimited"));
  assert.ok(canon.startsWith("_:0 "), canon);
});
