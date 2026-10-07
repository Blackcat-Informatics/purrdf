// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: asserts the strings, numbers and undefined the built wasm package hands a JavaScript caller, and that BigInt reads them exactly

// Exact XSD integers and decimals through the built package. A JavaScript `number`
// keeps 53 bits, so a value past it survives only as text: `xsdCanonicalLexical` and
// `xsdValueCompare` keep the value in the engine and return its canonical text or an
// ordering, and a SPARQL result carries the exact lexical form. Each refusal sits beside
// the valid neighbour it must keep.

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, Dataset, xsdCanonicalLexical, xsdValueCompare } from "../index.mjs";

await ready();

const XSD = "http://www.w3.org/2001/XMLSchema#";
const INTEGER = `${XSD}integer`;
const DECIMAL = `${XSD}decimal`;
const DOUBLE = `${XSD}double`;
const STRING = `${XSD}string`;

const I128_MAX = "170141183460469231731687303715884105727";
const PAST_I128 = "170141183460469231731687303715884105728";
const SIXTY = "123456789012345678901234567890123456789012345678901234567890";
const SIXTY_NEXT = "123456789012345678901234567890123456789012345678901234567891";

test("an integer past i128 has its exact canonical lexical form", () => {
  assert.equal(xsdCanonicalLexical(PAST_I128, INTEGER), PAST_I128);
  assert.equal(BigInt(xsdCanonicalLexical(PAST_I128, INTEGER)), 2n ** 127n);
  assert.equal(xsdCanonicalLexical(SIXTY, INTEGER), SIXTY);
  assert.equal(xsdCanonicalLexical(`+000${SIXTY}`, INTEGER), SIXTY);
});

test("a long decimal has its exact canonical lexical form", () => {
  const forty = "0.1000000000000000000000000000000000000001";
  assert.equal(xsdCanonicalLexical(forty, DECIMAL), forty);
  assert.equal(xsdCanonicalLexical("1.50", DECIMAL), "1.5");
});

test("a malformed lexical has no canonical form, and its well-formed neighbour does", () => {
  assert.equal(xsdCanonicalLexical("12x", INTEGER), undefined);
  assert.equal(xsdCanonicalLexical("12", INTEGER), "12");
  assert.equal(xsdCanonicalLexical("12", "http://example.org/datatype"), undefined);
});

test("long integers compare exactly", () => {
  assert.equal(xsdValueCompare(SIXTY, INTEGER, SIXTY_NEXT, INTEGER), -1);
  assert.equal(xsdValueCompare(SIXTY_NEXT, INTEGER, SIXTY, INTEGER), 1);
  assert.equal(xsdValueCompare(SIXTY, INTEGER, `0${SIXTY}`, INTEGER), 0);
  // 10^42 + 1 and 10^42 + 2 are the same JavaScript number.
  const plusOne = `1${"0".repeat(41)}1`;
  const plusTwo = `1${"0".repeat(41)}2`;
  assert.equal(Number(plusOne), Number(plusTwo));
  assert.equal(xsdValueCompare(plusOne, INTEGER, plusTwo, INTEGER), -1);
  assert.equal(xsdValueCompare(plusOne, INTEGER, `${plusOne}.0`, DECIMAL), 0);
});

test("malformed and incomparable values have no order, and their neighbours do", () => {
  assert.equal(xsdValueCompare("12x", INTEGER, "12", INTEGER), undefined);
  assert.equal(xsdValueCompare("12", INTEGER, "12", INTEGER), 0);
  assert.equal(xsdValueCompare("NaN", DOUBLE, "1", DOUBLE), undefined);
  assert.equal(xsdValueCompare("1", INTEGER, "1", STRING), undefined);
  assert.equal(xsdValueCompare("1", INTEGER, "1", DOUBLE), 0);
});

test("a SPARQL sum past i128 returns its exact lexical form", () => {
  const ds = new Dataset();
  const json = JSON.parse(ds.query(`SELECT ?n WHERE { BIND(${I128_MAX} + 1 AS ?n) }`));
  const [binding] = json.results.bindings;
  assert.equal(binding.n.type, "literal");
  assert.equal(binding.n.datatype, INTEGER);
  assert.equal(binding.n.value, PAST_I128);
  assert.equal(BigInt(binding.n.value), 2n ** 127n);
});
