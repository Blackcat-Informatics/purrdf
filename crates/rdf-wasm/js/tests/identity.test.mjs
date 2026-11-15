// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution conformance for the graph-IDENTITY surface reached through the
// PUBLIC package root — `Dataset.canonicalize()` (RDFC-1.0 canonical N-Quads) and
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
// `urn:purrdf:rdfc:` is the canonicalization overlay's OWN reserved namespace (see
// crates/rdf-core/src/ir/canon.rs). `canonicalize()` writes the flat assertion
// presentation, which writes no overlay sentinel and so applies no reservation: a
// reserved IRI outside the two fold shapes is an ordinary IRI there and is written
// as it is. The overlay presentation, reached through `isomorphic()`, still refuses it.
const RESERVED_PREDICATE_PLAIN_OBJECT = `@prefix ex: <http://example.org/> .
ex:r <urn:purrdf:rdfc:reifies> ex:o .
`;

// The valid neighbour: an IRI adjacent to the reserved namespace (a DIFFERENT
// `urn:purrdf:` sub-namespace, not `urn:purrdf:rdfc:`) in the exact same shape.
// It canonicalizes too, and the IRI survives into the output untouched.
const NEIGHBOURING_ORDINARY_PREDICATE = `@prefix ex: <http://example.org/> .
ex:r <urn:purrdf:other:annotation> ex:o .
`;

test("canonicalize() writes a reserved-vocabulary IRI as an ordinary IRI", () => {
  const admitted = Dataset.parse(RESERVED_PREDICATE_PLAIN_OBJECT, "turtle");
  const canon = canonicalValue(admitted.canonicalize("unlimited"));
  assert.ok(canon.includes("<urn:purrdf:rdfc:reifies>"), canon);
});

test("an ordinary IRI neighbouring the reserved namespace still canonicalizes", () => {
  const ok = Dataset.parse(NEIGHBOURING_ORDINARY_PREDICATE, "turtle");
  const canon = canonicalValue(ok.canonicalize("unlimited"));
  assert.ok(canon.includes("urn:purrdf:other:annotation"), canon);
});
