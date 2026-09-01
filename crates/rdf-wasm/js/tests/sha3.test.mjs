// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Node real-execution coverage for the SEP-0008 SHA-3 built-ins, driven through the
// ACTUAL compiled wasm module — query text in, SPARQL-results out.
//
// The Rust evaluator's unit tests pin these digests already. They cannot pin what a
// JavaScript host receives: between the query string and the object a browser reads
// there is a lexer that must keep `SHA3-256` as ONE word, a dispatch table that must
// send each size to its own digest, and a results encoder crossing the wasm boundary.
// This file exercises that whole path.

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, Dataset, QueryEngine } from "../index.mjs";

// One-time wasm instantiation before any test runs.
await ready();

// The NIST FIPS 202 example message `"abc"`, as one N-Triples statement.
const DATA = '<https://example.org/s> <https://example.org/message> "abc" .\n';

// [function name, SELECT alias, published FIPS 202 digest of "abc"].
//
// Provenance: NIST FIPS 202 publishes `"abc"` as a worked example for all four SHA-3
// sizes. Each value below was taken from that table and independently cross-checked
// against two implementations that are not the code under test — OpenSSL
// (`printf 'abc' | openssl dgst -sha3-256`) and CPython's `hashlib`
// (`hashlib.new("sha3_256", b"abc").hexdigest()`).
const VECTORS = [
  ["SHA3-224", "h224", "e642824c3f8cf24ad09234ee7d3c766fc9a3a5168d0c94ad73b46fdf"],
  ["SHA3-256", "h256", "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"],
  [
    "SHA3-384",
    "h384",
    "ec01498288516fc926459f58e2c6ad8df9b473cb0fc08c2596da7cf0e49be4b298d88cea927ac7f539f1edf228376d25",
  ],
  [
    "SHA3-512",
    "h512",
    "b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0",
  ],
];

// A SELECT projecting all four digests of ?m, with `spell` applied to each name.
function sha3Select(spell = (name) => name) {
  const projections = VECTORS.map(([name, alias]) => `(${spell(name)}(?m) AS ?${alias})`).join(" ");
  return `PREFIX ex: <https://example.org/> SELECT ${projections} WHERE { ?s ex:message ?m }`;
}

// The one solution row of `sha3Select(spell)`, from the raw JSON surface.
function sha3Row(spell) {
  const ds = Dataset.parse(DATA, "ntriples");
  const json = JSON.parse(ds.query(sha3Select(spell)));
  assert.equal(json.results.bindings.length, 1, "the fixture binds exactly one row");
  return json.results.bindings[0];
}

test("SHA-3 built-ins reach their published FIPS 202 digests through the wasm module", () => {
  const row = sha3Row();
  for (const [name, alias, want] of VECTORS) {
    assert.equal(row[alias].type, "literal", `${name} must come back as a literal`);
    assert.equal(row[alias].value, want, `${name} does not match its published FIPS 202 vector`);
  }
  // Distinct sizes: 224/256/384/512 bits are 56/64/96/128 hex characters.
  assert.deepEqual(
    VECTORS.map(([, alias]) => row[alias].value.length),
    [56, 64, 96, 128],
  );
});

test("SEP-0008's underscored spelling reaches the same digests through the wasm module", () => {
  const row = sha3Row((name) => name.replace("-", "_"));
  for (const [name, alias, want] of VECTORS) {
    assert.equal(row[alias].value, want, `${name} spelled with an underscore must agree`);
  }
});

test("SHA-3 digests survive the typed QueryEngine.select surface", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(DATA, "ntriples");
  const result = engine.select(ds, sha3Select());
  assert.equal(result.kind, "select");
  assert.deepEqual(result.variables, VECTORS.map(([, alias]) => alias));
  assert.equal(result.rowCount, 1);
  const row = result.rows.take(0);
  for (const [name, alias, want] of VECTORS) {
    assert.equal(row[alias].termType, "Literal", `${name} must be a Literal term`);
    assert.equal(row[alias].value, want, `${name} must survive the typed surface intact`);
  }
});

test("the SHA-3 hyphen is part of the name, and a spaced hyphen still subtracts", () => {
  const ds = Dataset.parse(DATA, "ntriples");
  // `SHA3` alone is no function: the spaced form is a hard parse failure that throws
  // into JavaScript rather than answering something else.
  assert.throws(() =>
    ds.query("PREFIX ex: <https://example.org/> SELECT (SHA3 - 256 AS ?h) WHERE { ?s ex:message ?m }"),
  );

  // And a genuine subtraction beside a SHA-3 call is still a subtraction: 64 - 4.
  const json = JSON.parse(
    ds.query(
      "PREFIX ex: <https://example.org/> " +
        "SELECT (STRLEN(SHA3-256(?m)) - 4 AS ?n) WHERE { ?s ex:message ?m }",
    ),
  );
  assert.equal(json.results.bindings[0].n.value, "60");
});
