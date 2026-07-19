// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Node real-execution conformance for the purrdf wasm package: drives the ACTUAL
// compiled wasm through the public RDF/JS surface, including the RDF-1.2 wedge
// (directional literals + quoted-triple terms) that no incumbent RDF/JS library has.

import { test } from "node:test";
import assert from "node:assert/strict";

import {
  ready,
  DataFactory,
  Dataset,
  Sink,
  version,
  datasetToStream,
  streamToDataset,
} from "../index.mjs";

// One-time wasm instantiation before any test runs.
await ready();

const XSD_INTEGER = "http://www.w3.org/2001/XMLSchema#integer";
const RDF_LANG_STRING = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const RDF_DIR_LANG_STRING = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

test("version() returns the crate semver", () => {
  assert.match(version(), /^\d+\.\d+\.\d+/);
});

test("DataFactory builds RDF/JS terms", () => {
  const f = new DataFactory();
  const n = f.namedNode("https://e/s");
  assert.equal(n.termType, "NamedNode");
  assert.equal(n.value, "https://e/s");

  const plain = f.literal("hi");
  assert.equal(plain.termType, "Literal");
  assert.equal(plain.datatype.value, "http://www.w3.org/2001/XMLSchema#string");

  const lang = f.literal("hi", "en");
  assert.equal(lang.language, "en");
  assert.equal(lang.datatype.value, RDF_LANG_STRING);
});

test("polymorphic literal(value, datatype) dispatches to a typed literal", () => {
  const f = new DataFactory();
  const xsdInteger = f.namedNode(XSD_INTEGER);
  const typed = f.literal("42", xsdInteger);
  assert.equal(typed.value, "42");
  assert.equal(typed.datatype.value, XSD_INTEGER);
});

test("parse → serialize → reparse round-trips N-Triples", () => {
  const input = "<https://e/s> <https://e/p> <https://e/o> .\n";
  const ds = Dataset.parse(input, "ntriples");
  assert.equal(ds.size, 1);
  const out = ds.serialize("ntriples");
  const reparsed = Dataset.parse(out, "ntriples");
  assert.equal(reparsed.size, 1);
});

test("expanded JSON-LD and YAML-LD wasm bytes are frozen", () => {
  const input = '<https://example.org/alice> <https://schema.org/name> "Alice" .\n';
  const ds = Dataset.parse(input, "nquads");
  assert.equal(
    ds.serialize("jsonld"),
    `{
  "@context": {},
  "@graph": [
    {
      "@id": "https://example.org/alice",
      "https://schema.org/name": {
        "@value": "Alice"
      }
    }
  ]
}`,
  );
  assert.equal(
    ds.serialize("yamlld"),
    `# yaml-language-server: $schema=purrdf.schema.json
# The default reference is the bundled purrdf.schema.json; pass an explicit
# schema_url to point editors at a hosted copy.
'@context': {}
'@graph':
- '@id': https://example.org/alice
  https://schema.org/name:
    '@value': Alice
`,
  );
});

test("DatasetCore add/has/delete/match/iterate", () => {
  const f = new DataFactory();
  const q1 = f.quad(
    f.namedNode("https://e/s1"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o1"),
  );
  const q2 = f.quad(
    f.namedNode("https://e/s2"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o2"),
  );
  const ds = new Dataset();
  // RDF/JS add/delete return the dataset instance for chaining; the "changed?" bit is
  // observed via size, not a return value.
  assert.equal(ds.add(q1), ds, "add returns the dataset instance (RDF/JS)");
  assert.equal(ds.size, 1);
  ds.add(q1); // idempotent
  assert.equal(ds.size, 1, "re-adding the same quad does not grow the set");
  ds.add(q2);
  assert.equal(ds.size, 2);
  assert.equal(ds.has(q1), true);

  const matched = ds.match(f.namedNode("https://e/s1"));
  assert.equal(matched.size, 1);

  // Iterable (the wrapper's Symbol.iterator over quads()).
  const subjects = [...ds].map((q) => q.subject.value).sort();
  assert.deepEqual(subjects, ["https://e/s1", "https://e/s2"]);

  assert.equal(ds.delete(q1), ds, "delete returns the dataset instance (RDF/JS)");
  assert.equal(ds.size, 1);
});

test("DatasetCore.add/delete chain (RDF/JS return-this)", () => {
  const f = new DataFactory();
  const q1 = f.quad(
    f.namedNode("https://e/s1"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o1"),
  );
  const q2 = f.quad(
    f.namedNode("https://e/s2"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o2"),
  );
  const ds = new Dataset();
  // The spec's headline use case: chained mutation.
  assert.equal(ds.add(q1).add(q2), ds, "add() chains and returns the dataset");
  assert.equal(ds.size, 2);
  assert.equal(ds.delete(q1).delete(q2), ds, "delete() chains and returns the dataset");
  assert.equal(ds.size, 0);
});

test("DatasetCore.match treats a Variable as a wildcard (RDF/JS idiom)", () => {
  const f = new DataFactory();
  const q1 = f.quad(
    f.namedNode("https://e/s1"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o1"),
  );
  const q2 = f.quad(
    f.namedNode("https://e/s2"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o2"),
  );
  const ds = new Dataset();
  ds.add(q1);
  ds.add(q2);

  // A Variable in any slot is a wildcard (must NOT throw, must NOT constrain) — here in
  // the subject, predicate, object, AND graph slots simultaneously.
  const all = ds.match(
    f.variable("s"),
    f.variable("p"),
    f.variable("o"),
    f.variable("g"),
  );
  assert.equal(all.size, 2);

  // A Variable graph term is Any (not a named-graph lookup that would throw), composed
  // with a concrete predicate constraint.
  const byPredicate = ds.match(
    f.variable("s"),
    f.namedNode("https://e/p"),
    undefined,
    f.variable("g"),
  );
  assert.equal(byPredicate.size, 2);
});

test("RDF-1.2 wedge — directional literal round-trips through N-Quads", () => {
  const f = new DataFactory();
  const dir = f.directionalLiteral("مرحبا", "ar", "rtl");
  assert.equal(dir.direction, "rtl");
  const ds = new Dataset();
  ds.add(f.quad(f.namedNode("https://e/s"), f.namedNode("https://e/p"), dir));
  const out = ds.serialize("nquads");
  const reparsed = Dataset.parse(out, "nquads");
  assert.equal(reparsed.size, 1);
  const obj = reparsed.quads()[0].object;
  assert.equal(obj.termType, "Literal");
  assert.equal(obj.language, "ar");
  assert.equal(obj.direction, "rtl");
});

test("RDF-1.2 wedge — quoted-triple term round-trips through N-Quads", () => {
  const f = new DataFactory();
  const quoted = f.quotedTriple(
    f.namedNode("https://e/s"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o"),
  );
  assert.equal(quoted.termType, "Quad");
  const ds = new Dataset();
  ds.add(
    f.quad(f.namedNode("https://e/stmt"), f.namedNode("https://e/asserts"), quoted),
  );
  const out = ds.serialize("nquads");
  const reparsed = Dataset.parse(out, "nquads");
  assert.equal(reparsed.size, 1);
  assert.equal(reparsed.quads()[0].object.termType, "Quad");
});

test("Sink streams quads into a dataset", () => {
  const f = new DataFactory();
  const sink = new Sink();
  sink.push(
    f.quad(f.namedNode("https://e/s"), f.namedNode("https://e/p"), f.namedNode("https://e/o")),
  );
  const ds = sink.finish();
  assert.equal(ds.size, 1);
});

test("datasetToStream → streamToDataset round-trips via the Sink", async () => {
  const f = new DataFactory();
  const ds = new Dataset();
  ds.add(f.quad(f.namedNode("https://e/s"), f.namedNode("https://e/p"), f.namedNode("https://e/o")));
  const rebuilt = await streamToDataset(datasetToStream(ds));
  assert.equal(rebuilt.size, 1);
});

test("an unsupported format is a rejected error", () => {
  assert.throws(() => Dataset.parse("", "yaml-ld"));
});

// RDF-1.2 directional-literal datatype reporting: the engine STORES rdf:langString
// (with the base direction in a separate identity field, matching how it interns a
// parsed literal), while the .datatype getter DERIVES the RDF-1.2 effective datatype
// rdf:dirLangString from the presence of a direction. The two surfaces are deliberately
// distinct: storage is the lookup key, reporting is the spec-correct view.
test("RDF-1.2 — directional literal: .datatype.value derives rdf:dirLangString", () => {
  const f = new DataFactory();
  const dir = f.directionalLiteral("مرحبا", "ar", "rtl");
  assert.equal(
    dir.datatype.value,
    RDF_DIR_LANG_STRING,
    "directional literal .datatype.value must be rdf:dirLangString",
  );
  assert.equal(dir.language, "ar");
  assert.equal(dir.direction, "rtl");
  // A plain language-tagged literal must still report rdf:langString.
  const langOnly = f.literal("مرحبا", "ar");
  assert.equal(langOnly.datatype.value, RDF_LANG_STRING);
});

test("RDF-1.2 — directional literal: in-memory add/has without serialize round-trip", () => {
  const f = new DataFactory();
  const s = f.namedNode("https://e/s");
  const p = f.namedNode("https://e/p");
  // Build the directional literal twice, independently: if the lookup key (the stored
  // RdfLiteral fed into the value→id lookup) diverges between two factory calls, the
  // TermValues mismatch and has() returns false.
  const dir1 = f.directionalLiteral("مرحبا", "ar", "rtl");
  const dir2 = f.directionalLiteral("مرحبا", "ar", "rtl");
  const q1 = f.quad(s, p, dir1);
  const q2 = f.quad(s, p, dir2);

  const ds = new Dataset();
  ds.add(q1);
  // has() must find the quad via a separately-constructed but value-identical term.
  assert.equal(
    ds.has(q2),
    true,
    "has() must return true for a separately-built identical directional literal",
  );
  // The dataset reports the stored literal's effective datatype as rdf:dirLangString.
  const stored = ds.quads()[0].object;
  assert.equal(
    stored.datatype.value,
    RDF_DIR_LANG_STRING,
    "the stored literal's .datatype.value must be rdf:dirLangString",
  );
  assert.equal(stored.direction, "rtl");
  assert.equal(stored.language, "ar");
});

// CROSS-PATH regression (the adversarial case): a directional literal PARSED from text
// is interned by the engine as rdf:langString + a separate direction. A factory-built
// identical literal (whose lookup key is also rdf:langString + direction) must be found
// by has(). If canonicalize_literal were to stamp rdf:dirLangString into the lookup key,
// this MISSES (datatype-string mismatch against the parse-interned langString).
test("RDF-1.2 — directional literal: parse then factory-built has() (cross-path)", () => {
  const input = '<https://e/s> <https://e/p> "مرحبا"@ar--rtl .\n';
  const ds = Dataset.parse(input, "nquads");
  assert.equal(ds.size, 1);

  const f = new DataFactory();
  const dir = f.directionalLiteral("مرحبا", "ar", "rtl");
  const query = f.quad(f.namedNode("https://e/s"), f.namedNode("https://e/p"), dir);
  assert.equal(
    ds.has(query),
    true,
    "a factory-built directional literal must match the parse-interned one (cross-path)",
  );
  // The parsed literal reports the RDF-1.2 effective datatype via the getter.
  const obj = ds.quads()[0].object;
  assert.equal(obj.datatype.value, RDF_DIR_LANG_STRING);
  assert.equal(obj.direction, "rtl");

  // RDF-1.2 inequality: a plain (non-directional) langString literal of the same text
  // and language must NOT be has()-equal to the directional one.
  const plain = f.literal("مرحبا", "ar");
  const plainQuery = f.quad(f.namedNode("https://e/s"), f.namedNode("https://e/p"), plain);
  assert.equal(
    ds.has(plainQuery),
    false,
    "a plain langString literal must NOT match a directional one (RDF-1.2 distinguishes them)",
  );
});

// --- RDF/JS spec conformance: Term.equals and Quad.equals with null/undefined ---

test("Term.equals(null) returns false (RDF/JS spec)", () => {
  const f = new DataFactory();
  const n = f.namedNode("https://e/s");
  assert.equal(n.equals(null), false, "Term.equals(null) must return false");
});

test("Term.equals(undefined) returns false (RDF/JS spec)", () => {
  const f = new DataFactory();
  const n = f.namedNode("https://e/s");
  assert.equal(n.equals(undefined), false, "Term.equals(undefined) must return false");
});

test("Term.equals sanity: same term is true, different term is false", () => {
  const f = new DataFactory();
  const a = f.namedNode("https://e/x");
  const b = f.namedNode("https://e/x");
  const c = f.namedNode("https://e/y");
  assert.equal(a.equals(b), true, "Term.equals(sameTerm) must be true");
  assert.equal(a.equals(c), false, "Term.equals(differentTerm) must be false");
});

test("Quad.equals(null) returns false (RDF/JS spec)", () => {
  const f = new DataFactory();
  const q = f.quad(
    f.namedNode("https://e/s"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o"),
  );
  assert.equal(q.equals(null), false, "Quad.equals(null) must return false");
});

test("Quad.equals(undefined) returns false (RDF/JS spec)", () => {
  const f = new DataFactory();
  const q = f.quad(
    f.namedNode("https://e/s"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o"),
  );
  assert.equal(q.equals(undefined), false, "Quad.equals(undefined) must return false");
});

test("Quad.equals sanity: same quad is true, different quad is false", () => {
  const f = new DataFactory();
  const q1 = f.quad(
    f.namedNode("https://e/s"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o"),
  );
  const q2 = f.quad(
    f.namedNode("https://e/s"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o"),
  );
  const q3 = f.quad(
    f.namedNode("https://e/s"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/DIFFERENT"),
  );
  assert.equal(q1.equals(q2), true, "Quad.equals(sameQuad) must be true");
  assert.equal(q1.equals(q3), false, "Quad.equals(differentQuad) must be false");
});

// NON-CONSUMPTION regression: a comparison MUST NOT consume its argument. If equals()
// takes a #[wasm_bindgen] struct BY VALUE, the generated glue moves the object into Rust
// and zeroes the JS handle — any later use of the argument throws "null pointer passed to
// rust". RDF/JS comparison must be read-only; the argument must remain fully usable after.
test("Term.equals does not consume its argument", () => {
  const f = new DataFactory();
  const a = f.namedNode("https://e/x");
  const b = f.namedNode("https://e/x");
  assert.equal(a.equals(b), true);
  // b MUST still be usable after being passed to equals().
  assert.equal(b.value, "https://e/x", "b.value must work after a.equals(b)");
  assert.equal(b.termType, "NamedNode", "b.termType must work after a.equals(b)");
  assert.equal(b.equals(a), true, "b.equals(a) must work after a.equals(b)");
});

test("Quad.equals does not consume its argument", () => {
  const f = new DataFactory();
  const qa = f.quad(
    f.namedNode("https://e/s"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o"),
  );
  const qb = f.quad(
    f.namedNode("https://e/s"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o"),
  );
  assert.equal(qa.equals(qb), true);
  // qb MUST still be usable after being passed to equals() — read accessors AND
  // dataset insertion (a second downstream consumer of the same handle).
  assert.equal(qb.subject.value, "https://e/s", "qb.subject must work after qa.equals(qb)");
  assert.equal(qb.equals(qa), true, "qb.equals(qa) must work after qa.equals(qb)");
  const ds = new Dataset();
  ds.add(qb);
  assert.equal(ds.has(qb), true, "dataset.add(qb) must succeed after qa.equals(qb)");
});
