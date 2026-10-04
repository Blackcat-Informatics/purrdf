// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Typed presentations on SPARQL parse failures, through the built package. A refused
// query or update throws (or rejects with) the same `Error`, with the same `message` and
// `code`, and also carries `presentation`: the parser's stable `sparql-parse-*`
// identity and typed parameters (exact integers as decimal strings), with an IRI
// refusal's own `iri-*` condition nested as `detail`. A failure without one carries no
// `presentation`.

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, Dataset, QueryEngine } from "../index.mjs";

await ready();

const EX = "http://example.org/";
const CDT_MAP = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map";
const DATA = `<${EX}s> <${EX}p> <${EX}o> .`;

const unsigned = (value) => ({ kind: "unsigned", value });
const text = (value) => ({ kind: "text", value });

// One case per parse-failure identity, with the exact message it has always thrown.
const QUERY_CASES = [
  {
    query: 'SELECT * WHERE { ?s ?p "unterminated }',
    messageId: "sparql-parse-lex",
    parameters: { at: unsigned("23"), reason: text("unterminated string literal") },
    english: "SPARQL lex error at byte 23: unterminated string literal",
  },
  {
    query: "ASK {",
    messageId: "sparql-parse-syntax",
    parameters: { at: unsigned("5") },
    english: "SPARQL syntax error at byte 5: expected an RDF term, found None",
  },
  {
    query: "ASK {} ORDER BY ?x",
    messageId: "sparql-parse-unsupported",
    parameters: { feature: text("solution modifiers on ASK") },
    english:
      "unsupported SPARQL construct: solution modifiers on ASK is outside the SPARQL 1.2 " +
      "query language this processor implements",
  },
  {
    query: `SELECT * WHERE { <${EX}%zz> ?p ?o }`,
    messageId: "sparql-parse-iri",
    parameters: { lexical: text(`${EX}%zz`) },
    detail: {
      messageId: "iri-bad-percent-encoding",
      parameters: { offset: unsigned("19") },
    },
    english:
      `invalid IRI "${EX}%zz" in term position: iri-bad-percent-encoding: ` +
      "malformed percent-encoding at byte 19",
  },
  {
    query: `SELECT (<${CDT_MAP}>("key") AS ?m) WHERE {}`,
    messageId: "sparql-parse-cdt-arity",
    parameters: { at: unsigned("57"), found: unsigned("1") },
    english:
      `SPARQL syntax error at byte 57: <${CDT_MAP}> takes an even number of arguments ` +
      "(key/value pairs), not 1",
  },
];

function thrown(call) {
  try {
    call();
  } catch (error) {
    return error;
  }
  assert.fail("the call must throw");
}

async function rejection(promise) {
  try {
    await promise;
  } catch (error) {
    return error;
  }
  assert.fail("the promise must reject");
}

function assertPresentation(error, { messageId, parameters, detail }) {
  assert.ok(error instanceof Error);
  assert.equal(error.presentation.messageId, messageId);
  for (const [name, value] of Object.entries(parameters)) {
    assert.deepEqual(error.presentation.parameters[name], value, name);
  }
  if (detail === undefined) {
    assert.equal(error.presentation.detail, undefined);
  } else {
    assert.equal(error.presentation.detail.messageId, detail.messageId);
    assert.deepEqual(error.presentation.detail.parameters, detail.parameters);
  }
}

for (const expected of QUERY_CASES) {
  test(`a refused query carries ${expected.messageId} on both lanes`, async () => {
    const engine = new QueryEngine();
    const ds = Dataset.parse(DATA, "ntriples");
    const sync = thrown(() => engine.query(ds, expected.query));
    assert.equal(sync.code, "native-sparql-query-parse");
    assert.equal(sync.message, `error native-sparql-query-parse: ${expected.english}`);
    assertPresentation(sync, expected);
    const viaDataset = thrown(() => ds.query(expected.query));
    assert.equal(viaDataset.message, sync.message);
    assert.deepEqual(viaDataset.presentation, sync.presentation);
    const async = await rejection(engine.queryAsync(ds, expected.query));
    assert.equal(async.code, sync.code);
    assert.equal(async.message, sync.message);
    assert.deepEqual(async.presentation, sync.presentation);
  });
}

test("a relative IRI without a base names its typed cause", () => {
  const error = thrown(() => new QueryEngine().query(Dataset.parse(DATA, "ntriples"), "SELECT * WHERE { <relative> ?p ?o }"));
  assertPresentation(error, {
    messageId: "sparql-parse-iri",
    parameters: { lexical: text("relative") },
    detail: { messageId: "iri-relative-no-base", parameters: { reference: text("relative") } },
  });
});

test("a refused update carries its typed presentation on both lanes", async () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(DATA, "ntriples");
  const sync = thrown(() => engine.update(ds, "DELETE WHERE {"));
  assert.equal(sync.code, "native-sparql-update-parse");
  assert.equal(
    sync.message,
    "error native-sparql-update-parse: SPARQL syntax error at byte 14: expected an RDF term, found None",
  );
  assertPresentation(sync, { messageId: "sparql-parse-syntax", parameters: { at: unsigned("14") } });
  const async = await rejection(engine.updateAsync(ds, "DELETE WHERE {"));
  assert.equal(async.message, sync.message);
  assert.deepEqual(async.presentation, sync.presentation);
  const iri = thrown(() => engine.update(ds, `INSERT DATA { <${EX}%zz> <${EX}p> 1 }`));
  assertPresentation(iri, {
    messageId: "sparql-parse-iri",
    parameters: {},
    detail: { messageId: "iri-bad-percent-encoding", parameters: { offset: unsigned("19") } },
  });
});

test("a failure without a typed presentation carries none; the valid neighbour answers", async () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(DATA, "ntriples");
  const service = "SELECT * WHERE { SERVICE <https://example.org/sparql> { ?s ?p ?o } }";
  const sync = thrown(() => engine.query(ds, service));
  assert.equal(sync.code, "native-sparql-service-unconfigured");
  assert.equal("presentation" in sync, false);
  const async = await rejection(engine.queryAsync(ds, service));
  assert.equal(async.code, "native-sparql-service-unconfigured");
  assert.equal("presentation" in async, false);
  assert.equal(engine.query(ds, "ASK {}").boolean, true);
  assert.equal(engine.query(ds, `SELECT * WHERE { <${EX}s> ?p ?o }`).kind, "select");
});
