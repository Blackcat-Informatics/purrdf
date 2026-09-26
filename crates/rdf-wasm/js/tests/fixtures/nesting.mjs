// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Shared by the nesting tests of both lanes (`query.test.mjs`, `async-concurrency.test.mjs`):
// five nesting shapes whose answers depend on every level being honoured, a runner that
// tells an answer from a typed stack refusal (and fails on anything else), and a bisection
// that finds where a lane's stacks end.

import assert from "node:assert/strict";

export const STACK_REFUSAL = /SPARQL parse stack exhausted|evaluation stack exhausted|stack region exhausted/;
export const NUMBERS = [1, 2]
  .map(
    (n) =>
      `<https://example.org/s${n}> <https://example.org/p> "${n}"^^<http://www.w3.org/2001/XMLSchema#integer> .`,
  )
  .join("\n");
const nest = (open, core, close, depth) => `${open.repeat(depth)}${core}${close.repeat(depth)}`;
// Each shape's answer depends on every level being honoured, so a truncated or dropped
// level would answer differently.
export const NESTING_SHAPES = [
  [
    "nested parentheses",
    (n) => `SELECT ?s WHERE { ?s <https://example.org/p> ?o FILTER(${nest("(", "?o", ")", n)} = 1) }`,
    ["s1"],
  ],
  // |1 - 3| = 2 and |2 - 3| = 1: only `s2` equals 1, and only if the innermost ABS applied.
  [
    "nested ABS(",
    (n) => `SELECT ?s WHERE { ?s <https://example.org/p> ?o FILTER(${nest("ABS(", "?o - 3", ")", n)} = 1) }`,
    ["s2"],
  ],
  // n negations of `?o` equal (-1)^n times it.
  [
    "nested -(",
    (n) =>
      `SELECT ?s WHERE { ?s <https://example.org/p> ?o FILTER(${nest("-(", "?o", ")", n)} = ${n % 2 ? -1 : 1}) }`,
    ["s1"],
  ],
  [
    "nested groups",
    (n) => `SELECT ?s WHERE { ${nest("{ ", "?s <https://example.org/p> ?o FILTER(?o = 1)", " }", n)} }`,
    ["s1"],
  ],
  [
    "nested property-path groups",
    (n) => `SELECT ?s WHERE { ?s ${nest("(", "<https://example.org/p>", ")", n)} ?o FILTER(?o = 2) }`,
    ["s2"],
  ],
];
const localNames = (result) =>
  result.rows
    .toArray()
    .map((row) => row.s.value.replace("https://example.org/", ""))
    .sort();

/** `run(query)` as `{ subjects }` or `{ refused }` — a typed stack refusal, asserted. */
export async function attempt(run, query, what) {
  try {
    return { subjects: localNames(await run(query)) };
  } catch (error) {
    assert.match(error.message, STACK_REFUSAL, `${what}: a typed stack refusal, not a trap`);
    return { refused: error.message };
  }
}

/** The deepest level of `shape` that answers under `run`, by bisection, and the refusal one level deeper. */
export async function realLimit(run, [what, text, expected]) {
  let [deepest, refused] = [1, 20_000];
  assert.deepEqual((await attempt(run, text(deepest), what)).subjects, expected, `${what}: one level answers`);
  assert.ok((await attempt(run, text(refused), what)).refused, `${what}: 20 000 levels are refused`);
  while (refused - deepest > 1) {
    const mid = (deepest + refused) >> 1;
    if ((await attempt(run, text(mid), what)).subjects) deepest = mid;
    else refused = mid;
  }
  assert.deepEqual((await attempt(run, text(deepest), what)).subjects, expected, `${what}: ${deepest} deep answers`);
  assert.ok((await attempt(run, text(refused), what)).refused, `${what}: ${refused} deep is refused`);
  return deepest;
}

