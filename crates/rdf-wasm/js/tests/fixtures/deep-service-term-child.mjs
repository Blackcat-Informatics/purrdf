// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { DataFactory, Dataset, QueryEngine, ready, streamToDataset } from "../../index.mjs";

await ready();
const EX = "https://example.org/";
const depth = 20_000;
const iri = `{"type":"uri","value":"${EX}a"}`;
const open = `{"type":"triple","value":{"subject":${iri},"predicate":${iri},"object":`;
const body = `{"head":{"vars":["x"]},"results":{"bindings":[{"x":${open.repeat(depth)}${iri}${"}}".repeat(depth)}}]}}`;
const engine = new QueryEngine();
const factory = new DataFactory();
const dataset = new Dataset();
for (const silent of [false, true]) {
  const result = await engine.queryAsync(dataset,
    `SELECT ?x WHERE { SERVICE ${silent ? "SILENT " : ""}<${EX}sparql> { ?s ?p ?x } }`,
    { resolveService: () => body });
  const term = result.rows.toArray()[0].x;
  const copy = factory.fromTerm(term);
  assert.ok(term.equals(copy));
  let at = factory.fromTerm(term);
  let levels = 0;
  while (at.termType === "Quad") {
    const predicate = at.predicate;
    assert.equal(predicate.value, `${EX}a`);
    predicate.free();
    const next = at.object;
    at.free();
    at = next;
    levels += 1;
  }
  assert.equal(levels, depth);
  assert.equal(at.value, `${EX}a`);
  at.free();
  const outer = factory.quad(factory.namedNode(`${EX}s`), factory.namedNode(`${EX}p`), term);
  await assert.rejects(streamToDataset([outer]), /rdf-ir-triple-nesting-limit/, "the event protocol refuses its depth limit safely");
  const shallow = factory.quad(factory.namedNode(`${EX}s`), factory.namedNode(`${EX}p`), factory.namedNode(`${EX}o`));
  const neighbour = await streamToDataset([shallow]);
  assert.equal(neighbour.size, 1);
  neighbour.free();
  shallow.free();
  const streamed = new Dataset();
  streamed.add(outer);
  assert.equal(streamed.size, 1, "deep terms can be inserted through the mutable boundary");
  assert.ok(streamed.has(outer));
  const reread = streamed.quads()[0].object;
  assert.ok(reread.equals(term), "dataset egress keeps the deep term");
  const matched = streamed.match(undefined, undefined, factory.fromTerm(term));
  assert.equal(matched.size, 1, "deep pattern matching never creates a recursively owned term");
  reread.free();
  matched.free();
  streamed.free();
  outer.free();
  copy.free();
  term.free();
  assert.equal(engine.select(dataset, "SELECT * WHERE {}").rowCount, 1, "the instance remains usable");
}
process.stdout.write(`${JSON.stringify({ depth, lanes: 2 })}\n`);
