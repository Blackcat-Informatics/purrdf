// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: executes the optimized shipped WebAssembly package through its JavaScript root API and observes actual host result objects.

import { test } from "node:test";
import assert from "node:assert/strict";
import { ready, Dataset, QueryEngine } from "../index.mjs";

await ready();

const ex = "https://example.org/";
const laws = ["xpath-2.0-2010-12-14", "xpath-3.1-2017-03-21"];
const prose = "gamma graph beta rdf pattern shape delta alpha node sparql ".repeat(80000).slice(0, 4 << 20);
const ab = "abbaab".repeat(Math.ceil((4 << 20) / 6)).slice(0, 4 << 20);
assert.equal(prose.length, 4 << 20);
assert.equal(ab.length, 4 << 20);
const values = {
  prose,
  prose_lt: `${prose}<`,
  prose_bang: `${prose}!`,
  ab,
  ab_longer: `${ab}a`,
  ab_broken: `${ab.slice(0, 3999999)}c${ab.slice(0, 194305)}`,
};
const cases = [
  [String.raw`^[\s\S]{0,10000000}$`, "prose", true],
  [String.raw`^[^\n]{0,10000000}$`, "prose", true],
  [String.raw`^[^<>]{1,5000000}$`, "prose", true],
  [String.raw`^[^<>]{1,5000000}$`, "prose_lt", false],
  [String.raw`^[\p{L} ]{1,5000000}$`, "prose", true],
  [String.raw`^(\w+\s?){1,5000000}$`, "prose", true],
  [String.raw`^(\w+\s?){1,5000000}$`, "prose_bang", false],
  ["^(a|b){4194304}$", "ab", true],
  ["^(a|b){4194304}$", "ab_longer", false],
  ["(a|b){4000000}", "ab", true],
  ["(a|b){4000000}", "ab_broken", false],
];

for (const law of laws) {
  test(`typed refusal and healthy nullable-count neighbor: ${law}`, () => {
    const dataset = Dataset.parse(`<${ex}s> <${ex}p> "aaab" .`, "nquads");
    const engine = new QueryEngine();
    const query = (pattern) => `SELECT ?r WHERE { <${ex}s> <${ex}p> ?v BIND(REGEX(?v, ${JSON.stringify(pattern)}) AS ?r) }`;
    const healthy = engine.select(dataset, query("((a?){3000}){3000}b"), { xpathRegex: law });
    assert.deepEqual(healthy.rows.toArray().map((row) => row.r.value), ["true"]);
    for (const pattern of ["((a?){5000}){5000}b", "a".repeat((64 << 10) + 1)]) {
      assert.throws(
        () => engine.select(dataset, query(pattern), { xpathRegex: law }),
        (error) => {
          assert.ok(error instanceof Error);
          assert.ok(
            pattern.length > (64 << 10)
              ? error.code === "xpath-pattern-bytes"
              : ["xpath-match-steps", "xpath-match-slots"].includes(error.code),
            `${law}: ${error.code}: ${error.message}`,
          );
          return true;
        },
      );
    }
    const retry = engine.select(dataset, query("a+b"), { xpathRegex: law });
    assert.deepEqual(retry.rows.toArray().map((row) => row.r.value), ["true"]);
  });

  test(`large finite REGEX shapes through the optimized package: ${law}`, () => {
    const dataset = Dataset.parse(
      Object.entries(values).map(([name, value]) => `<${ex}${name}> <${ex}p> ${JSON.stringify(value)} .`).join("\n"),
      "nquads",
    );
    const rows = cases.map(([pattern, subject], index) => `(${index} <${ex}${subject}> ${JSON.stringify(pattern)})`).join(" ");
    const result = new QueryEngine().select(
      dataset,
      `SELECT ?r WHERE { VALUES (?i ?s ?p) { ${rows} } ?s <${ex}p> ?v BIND(REGEX(?v, ?p) AS ?r) } ORDER BY ?i`,
      { xpathRegex: law },
    );
    assert.deepEqual(result.rows.toArray().map((row) => row.r.value), cases.map(([, , matched]) => String(matched)));
  });

  test(`large finite REPLACE retains the last capture: ${law}`, () => {
    const half = prose.slice(0, 2 << 20);
    const dataset = Dataset.parse(`<${ex}s> <${ex}p> ${JSON.stringify(half)} .`, "nquads");
    const pattern = JSON.stringify(String.raw`^(\w+\s?){1,5000000}$`);
    const result = new QueryEngine().select(
      dataset,
      `SELECT ?r WHERE { <${ex}s> <${ex}p> ?v BIND(REPLACE(?v, ${pattern}, "[$1]") AS ?r) }`,
      { xpathRegex: law },
    );
    assert.deepEqual(result.rows.toArray().map((row) => row.r.value), [`[${half.slice(half.trimEnd().lastIndexOf(" ") + 1)}]`]);
  });
}
