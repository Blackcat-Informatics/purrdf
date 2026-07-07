// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Node real-execution conformance for the graph-IDENTITY surface reached through the
// PUBLIC package root — `Dataset.canonicalize()` (RDFC-1.0 canonical N-Quads) and
// `Dataset.isomorphic(other)` (graph isomorphism under blank-node relabeling), exactly
// as the docs playground's identity/diff pane calls them.

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, Dataset } from "../index.mjs";

await ready();

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
  assert.equal(a.isomorphic(b), true);
});

test("isomorphic() is false for a structurally different graph", () => {
  const a = Dataset.parse(A, "turtle");
  const c = Dataset.parse(C, "turtle");
  assert.equal(a.isomorphic(c), false);
});

test("canonicalize() is stable and identifies isomorphic graphs byte-for-byte", () => {
  const a = Dataset.parse(A, "turtle");
  const b = Dataset.parse(B, "turtle");
  const canonA = a.canonicalize();
  assert.ok(canonA.length > 0, "canonical form is non-empty");
  // Re-canonicalizing the same dataset is deterministic.
  assert.equal(a.canonicalize(), canonA);
  // Isomorphic graphs canonicalize to the identical string — the identity guarantee.
  assert.equal(b.canonicalize(), canonA);
});
