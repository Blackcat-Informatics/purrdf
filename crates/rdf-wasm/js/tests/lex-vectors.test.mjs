// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { createHash } from "node:crypto";
import { Dataset, QueryEngine, ready } from "../index.mjs";
await ready();
const directory = new URL("../../../lex/tests/vectors/", import.meta.url);
const digest = text => createHash("sha256").update(text).digest("hex");
function vectors(name) {
  const text = readFileSync(new URL(name, directory), "utf8");
  const lines = text.split("\n").filter(line => line && !line.startsWith("#"));
  assert.equal(lines.length, Number(text.match(/^# vector-count: (\d+)$/m)[1]), name);
  assert.equal(digest(lines.join("\n") + "\n"), text.match(/^# body-sha256: ([0-9a-f]+)$/m)[1], name);
  return lines.map(line => line.split("\t"));
}
function decode(field) {
  if (field === "\\0") return "";
  return field.replace(/\\(\\|x[0-9a-f]{2})/g, (_, code) => code === "\\" ? "\\" : String.fromCharCode(parseInt(code.slice(1), 16)));
}
function encode(text) {
  if (!text) return "\\0";
  return text.replace(/[\\\x00-\x20#\x7f]/g, c => c === "\\" ? "\\\\" : `\\x${c.charCodeAt(0).toString(16).padStart(2, "0")}`);
}
function jsonValues(bodies) {
  const graph = bodies.map((body, index) => `{"@id":"https://example.org/s${index}","https://example.org/p":{"@value":"${body}"}}`).join(",");
  const ds = Dataset.parse(`{"@graph":[${graph}]}`, "jsonld");
  try {
    const result = Array(bodies.length);
    for (const q of ds.quads()) {
      const subject = q.subject; const object = q.object;
      result[Number(subject.value.slice("https://example.org/s".length))] = object.value;
      subject.free(); object.free();
      q.free();
    }
    assert.equal(ds.size, bodies.length);
    assert.equal(result.filter(value => typeof value === "string").length, bodies.length);
    return result;
  } finally { ds.free(); }
}
const inventory = {
  "json_string_vectors.txt": "JSON-LD literal parsing",
  "json_unit_vectors.txt": "JSON-LD literal parsing",
  "percent_unreserved_vectors.txt": "SPARQL ENCODE_FOR_URI",
  "escape_vectors.txt": "Turtle literal and IRIREF escape parsing with suffix boundaries",
  "uchar_scalar_vectors.txt": "Turtle literal UCHAR parsing",
  "find_byte_vectors.txt": "No exported byte-search primitive",
  "json_pointer_vectors.txt": "No exported pointer primitive",
  "unicode_normalization_vectors.txt": "No exported normalization or combining-class primitive",
  "normalization_differential_vectors.txt": "No exported normalization primitive",
};
for (const name of readdirSync(directory)) {
  if (name.startsWith("percent_") && !(name in inventory)) inventory[name] = "No exported encoder/decoder for this percent law";
}
test("frozen lexical capability inventory accounts for every file", () => {
  assert.deepEqual(Object.keys(inventory).sort(), readdirSync(directory).sort());
});
test("every frozen JSON string vector crosses the package decoder", () => {
  for (const [body, expected] of vectors("json_string_vectors.txt")) {
    if (expected === "-") assert.throws(() => jsonValues([decode(body)]), body);
    else assert.equal(encode("=" + jsonValues([decode(body)])[0]), expected, body);
  }
});
test("every JSON code unit and surrogate pair matches the frozen answer digest", () => {
  for (const [form, digitCase, first, last, expected] of vectors("json_unit_vectors.txt")) {
    const hash = createHash("sha256");
    const unit = cp => "\\u" + (digitCase === "upper" ? cp.toString(16).toUpperCase() : cp.toString(16)).padStart(4, "0");
    let bodies = [];
    function flush() {
      for (const value of jsonValues(bodies)) hash.update(encode("=" + value) + "\n");
      bodies = [];
    }
    for (let cp = parseInt(first, 16); cp <= parseInt(last, 16); cp++) {
      if (form === "unit" && cp >= 0xd800 && cp <= 0xdfff) {
        if (bodies.length) flush();
        assert.throws(() => jsonValues([unit(cp)])); hash.update("-\n");
      } else {
        const n = cp - 0x10000;
        bodies.push(form === "unit" ? unit(cp) : unit(0xd800 + (n >> 10)) + unit(0xdc00 + (n & 0x3ff)));
        if (bodies.length === 512) flush();
      }
    }
    if (bodies.length) flush();
    assert.equal(hash.digest("hex"), expected, `${form} ${digitCase} ${first}-${last}`);
  }
});
test("every unreserved percent vector crosses SPARQL ENCODE_FOR_URI", () => {
  const engine = new QueryEngine(); const ds = new Dataset();
  function encoded(inputs) {
    const tuples = inputs.map((value, i) => `(${i} ${JSON.stringify(value)})`).join(" ");
    const result = engine.select(ds, `SELECT ?i (ENCODE_FOR_URI(?s) AS ?v) WHERE { VALUES (?i ?s) { ${tuples} } }`);
    const out = Array(inputs.length);
    for (const row of result.rows) {
      out[Number(row.i.value)] = row.v.value;
      row.i.free(); row.v.free();
    }
    return out;
  }
  for (const [kind, input, expected] of vectors("percent_unreserved_vectors.txt")) {
    if (kind === "text") assert.equal(encoded([decode(input)])[0], decode(expected), input);
    else {
      const hash = createHash("sha256"); let inputs = [];
      function flush() { for (const value of encoded(inputs)) hash.update(value + "\n"); inputs = []; }
      const plane = parseInt(input, 16);
      for (let cp = plane * 0x10000; cp < (plane + 1) * 0x10000; cp++) {
        if (cp >= 0xd800 && cp <= 0xdfff) continue;
        inputs.push(String.fromCodePoint(cp)); if (inputs.length === 512) flush();
      }
      if (inputs.length) flush(); assert.equal(hash.digest("hex"), expected, `plane ${plane}`);
    }
  }
  ds.free(); engine.free();
});

function turtleValues(bodies) {
  const text = bodies.map((body, i) => `<https://example.org/s${i}> <https://example.org/p> "${body}" .`).join("\n");
  const ds = Dataset.parse(text, "turtle");
  try {
    const result = Array(bodies.length);
    for (const q of ds.quads()) {
      const subject = q.subject; const object = q.object;
      result[Number(subject.value.slice("https://example.org/s".length))] = object.value;
      subject.free(); object.free(); q.free();
    }
    assert.equal(result.filter(value => typeof value === "string").length, bodies.length);
    return result;
  } finally { ds.free(); }
}
test("every frozen string escape preserves its consumed-prefix boundary", () => {
  for (const [field, uchar, expected] of vectors("escape_vectors.txt")) {
    const input = decode(field);
    if (expected === "-") {
      // The longest primitive escape is ten bytes; literalize the suffix so its
      // independent syntax cannot manufacture a refusal of an otherwise valid prefix.
      const prefix = input.slice(0, 10);
      const suffix = JSON.stringify(input.slice(10)).slice(1, -1);
      assert.throws(() => turtleValues([prefix + suffix]), field);
    } else {
      const [scalar, n] = expected.split("/");
      const consumed = Number(n);
      const rest = input.slice(consumed);
      const body = input.slice(0, consumed) + JSON.stringify(rest).slice(1, -1);
      assert.equal(turtleValues([body])[0], String.fromCodePoint(parseInt(scalar, 16)) + rest, field);
      if (uchar !== "-") assert.equal(expected, uchar, field);
      else {
        // ECHAR is legal in a literal and prohibited in an IRIREF: the UCHAR-only seam.
        assert.throws(() => Dataset.parse(`<https://example.org/${input.slice(0, consumed)}> <https://example.org/p> "x" .`, "turtle"));
      }
    }
  }
});
test("every UCHAR scalar and refusal matches the frozen answer digests", () => {
  for (const [form, digitCase, first, last, expected] of vectors("uchar_scalar_vectors.txt")) {
    const hash = createHash("sha256"); let bodies = [];
    const consumed = form === "u" ? 6 : 10;
    const escape = cp => "\\" + form + (digitCase === "upper" ? cp.toString(16).toUpperCase() : cp.toString(16)).padStart(consumed - 2, "0");
    function flush() {
      for (const value of turtleValues(bodies)) {
        assert.equal([...value].length, 1);
        hash.update(value.codePointAt(0).toString(16).toUpperCase().padStart(4, "0") + `/${consumed}\n`);
      }
      bodies = [];
    }
    for (let cp = parseInt(first, 16); cp <= parseInt(last, 16); cp++) {
      if ((cp >= 0xd800 && cp <= 0xdfff) || cp > 0x10ffff) {
        if (bodies.length) flush();
        assert.throws(() => turtleValues([escape(cp)])); hash.update("-\n");
      } else { bodies.push(escape(cp)); if (bodies.length === 512) flush(); }
    }
    if (bodies.length) flush(); assert.equal(hash.digest("hex"), expected, `${form} ${digitCase} ${first}-${last}`);
  }
});
