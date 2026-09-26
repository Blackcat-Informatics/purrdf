// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution tests for the asynchronous SHACL twins — `shaclValidateToSarifAsync`,
// `shaclValidateChangesToSarifAsync`, `shaclEntailAsync` and the four
// `shaclProductValidateToSarif…Async` — driven through the package root against the
// actual optimized wasm module under WebAssembly JavaScript Promise Integration.
//
// SHACL evaluates SPARQL (`sh:SPARQLTarget` queries, SHACL-SPARQL constraints, SHACL-AF
// rules and node expressions), so each twin runs its synchronous twin's own body as a
// job: every query suspends on the host's SERVICE handler, the job yields and stops on
// its signal, and the signal is polled between focus nodes too. Every oracle compares an
// asynchronous answer with an independent observation — the synchronous twin's bytes, a
// report whose focus nodes a mock resolver's answer decides, or `queryAsync`'s rows for
// the same query — and every fixture pair is chosen so the compared values differ from
// case to case, so an equality cannot be satisfied by a twin that returns a constant.
//
// Wall-clock bounds are never asserted: the machine running this is not quiet.

import { test } from "node:test";
import assert from "node:assert/strict";

import {
  Dataset,
  QueryEngine,
  ready,
  shaclEntail,
  shaclEntailAsync,
  shaclPackProduct,
  shaclProductExplain,
  ShaclProductRefusal,
  shaclProductValidateToSarif,
  shaclProductValidateToSarifAsync,
  shaclProductValidateToSarifExpecting,
  shaclProductValidateToSarifExpectingAsync,
  shaclProductValidateToSarifRebuild,
  shaclProductValidateToSarifRebuildAsync,
  shaclProductValidateToSarifRebuildExpecting,
  shaclProductValidateToSarifRebuildExpectingAsync,
  shaclValidateChangesToSarif,
  shaclValidateChangesToSarifAsync,
  shaclValidateToSarif,
  shaclValidateToSarifAsync,
} from "../index.mjs";
import init from "../pkg/purrdf_wasm.js";

await ready();
// Already instantiated: `init` hands back the one instance's raw exports.
const exports = await init();
const stackPointer = () => exports.__wbindgen_add_to_stack_pointer(0) >>> 0;
const IDLE = stackPointer();

const EX = "http://example.org/";
const RDF_TYPE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const XSD_INTEGER = "http://www.w3.org/2001/XMLSchema#integer";
const ENDPOINT = `${EX}sparql`;

const PREFIXES = `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <${EX}> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
`;

// Core constraints only: read from the IR, no SPARQL.
const CORE_SHAPES = `${PREFIXES}
ex:PersonShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [ sh:path ex:age ; sh:datatype xsd:integer ; sh:minCount 1 ] .
`;

// A SHACL-SPARQL constraint: every nickname shorter than three characters violates.
const SPARQL_SHAPES = `${PREFIXES}
ex:NickShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:sparql [
    a sh:SPARQLConstraint ;
    sh:message "nickname too short" ;
    sh:select """
      SELECT $this ?value WHERE {
        $this <${EX}nickname> ?value .
        FILTER (STRLEN(?value) < 3)
      }
    """ ;
  ] .
`;

// A SPARQL-based TARGET that asks a remote registry which people are banned, and a
// constraint every person here violates (none has a clearance): the report's focus nodes
// are exactly the people the registry's answer names. The SERVICE is in the target
// because SHACL forbids it in a constraint's query, where `$this` is pre-bound.
const serviceShapes = (silent) => `${PREFIXES}
ex:BannedShape a sh:NodeShape ;
  sh:target [
    a sh:SPARQLTarget ;
    sh:select """
      SELECT ?this WHERE {
        ?this a <${EX}Person> .
        SERVICE ${silent ? "SILENT " : ""}<${ENDPOINT}> { ?this <${EX}status> "banned" }
      }
    """ ;
  ] ;
  sh:property [ sh:path ex:clearance ; sh:minCount 1 ] .
`;
const TARGET_QUERY = `SELECT ?this WHERE { ?this a <${EX}Person> . SERVICE SILENT <${ENDPOINT}> { ?this <${EX}status> "banned" } }`;

// A SHACL-AF triple rule, for the entailment twin.
const RULE_SHAPES = `${PREFIXES}
ex:AdultRule a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .
`;

/** `count` people; ages are integers except where `badAge(index)`, nicknames short where `shortNick(index)`. */
function people(count, { badAge = () => false, shortNick = () => false } = {}) {
  const lines = [];
  for (let index = 0; index < count; index += 1) {
    const person = `<${EX}p${index}>`;
    lines.push(`${person} <${RDF_TYPE}> <${EX}Person> .`);
    lines.push(
      badAge(index)
        ? `${person} <${EX}age> "unknown" .`
        : `${person} <${EX}age> "${20 + (index % 50)}"^^<${XSD_INTEGER}> .`,
    );
    lines.push(`${person} <${EX}nickname> "${shortNick(index) ? "n" : `nick${index}`}" .`);
  }
  return `${lines.join("\n")}\n`;
}

const CONFORMING = people(12);
const NONCONFORMING = people(12, { badAge: (i) => i % 4 === 0, shortNick: (i) => i % 3 === 0 });

// Two people, Alice and Bob.
const ALICE_AND_BOB = `<${EX}alice> <${RDF_TYPE}> <${EX}Person> .\n<${EX}bob> <${RDF_TYPE}> <${EX}Person> .\n`;

/** The registry's SPARQL Results JSON answer to the forwarded pattern, banning `who`. */
const registryAnswer = (...who) =>
  JSON.stringify({
    head: { vars: ["this"] },
    results: { bindings: who.map((name) => ({ this: { type: "uri", value: `${EX}${name}` } })) },
  });

/** How many results a SARIF log reports (a log with none omits the array). */
function resultCount(sarif) {
  return (JSON.parse(sarif).runs[0].results ?? []).length;
}

/** The focus nodes a SARIF log reports, sorted. */
function focusNodes(sarif) {
  const log = JSON.parse(sarif);
  return (log.runs[0].results ?? [])
    .flatMap((result) => result.locations ?? [])
    .flatMap((location) => location.logicalLocations ?? [])
    .filter((location) => location.kind === "focusNode")
    .map((location) => location.name)
    .sort();
}

/** A `resolveService` that records every call and answers with `answer(request, ctx)`. */
function recordingResolver(answer) {
  const calls = [];
  const resolveService = (request, ctx) => {
    calls.push({ request, ctx });
    return answer(request, ctx);
  };
  return { calls, resolveService };
}

/** Resolves to the rejection reason, or fails the test if `promise` resolves. */
async function rejection(promise) {
  try {
    await promise;
  } catch (error) {
    return error;
  }
  assert.fail("expected the promise to reject");
}

/** The synchronous throw of `fn`, or fails the test if it returns. */
function syncThrow(fn) {
  try {
    fn();
  } catch (error) {
    return error;
  }
  assert.fail("expected the call to throw");
}

/** The `identity-digest` line of a product. */
function identityDigestOf(product) {
  const line = shaclProductExplain(product)
    .split("\n")
    .find((candidate) => candidate.startsWith("identity-digest "));
  return line.slice("identity-digest ".length);
}

test("async SHACL twins return exactly their synchronous twins' results on core and SHACL-SPARQL fixtures, conforming and not", async () => {
  for (const [label, shapes, data, violations] of [
    ["core, conforming", CORE_SHAPES, CONFORMING, 0],
    ["core, non-conforming", CORE_SHAPES, NONCONFORMING, 3],
    ["sparql, conforming", SPARQL_SHAPES, CONFORMING, 0],
    ["sparql, non-conforming", SPARQL_SHAPES, NONCONFORMING, 4],
  ]) {
    const sync = shaclValidateToSarif(shapes, data);
    // The fixture is not vacuous: its violation count is the one the case names.
    assert.equal(resultCount(sync), violations, label);
    assert.equal(await shaclValidateToSarifAsync(shapes, data), sync, label);
    assert.equal(await shaclValidateToSarifAsync(shapes, data, null, { yieldEveryPolls: 0 }), sync, label);
  }
  // The reports differ from case to case, so the equalities above compared real reports.
  assert.notEqual(
    await shaclValidateToSarifAsync(SPARQL_SHAPES, NONCONFORMING),
    await shaclValidateToSarifAsync(CORE_SHAPES, NONCONFORMING),
  );

  // `shapesBase` is forwarded exactly as the synchronous twin forwards it.
  const relative = `${PREFIXES}<PersonShape> a sh:NodeShape ; sh:targetClass ex:Person ; sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .`;
  const based = shaclValidateToSarif(relative, NONCONFORMING, EX);
  assert.equal(await shaclValidateToSarifAsync(relative, NONCONFORMING, EX), based);
  const unbased = syncThrow(() => shaclValidateToSarif(relative, NONCONFORMING));
  assert.equal((await rejection(shaclValidateToSarifAsync(relative, NONCONFORMING))).message, unbased.message);

  // A parse error rejects with the synchronous twin's words, carrying the job's evidence.
  const malformed = syncThrow(() => shaclValidateToSarif(CORE_SHAPES, "<not n-triples"));
  const asyncMalformed = await rejection(shaclValidateToSarifAsync(CORE_SHAPES, "<not n-triples"));
  assert.equal(asyncMalformed.message, malformed.message);
  assert.equal(typeof asyncMalformed.evidence.async.polls, "number");
});

test("the change, entailment and product twins return their synchronous twins' values", async () => {
  // Change validation: a bounded change and an unbounded (SPARQL) one.
  const added = `<${EX}p1> <${EX}age> "old" .\n`;
  const removed = `<${EX}p2> <${EX}age> "22"^^<${XSD_INTEGER}> .\n`;
  for (const shapes of [CORE_SHAPES, SPARQL_SHAPES]) {
    const sync = shaclValidateChangesToSarif(shapes, CONFORMING, added, removed);
    const changed = await shaclValidateChangesToSarifAsync(shapes, CONFORMING, added, removed);
    try {
      assert.equal(changed.sarif, sync.sarif);
      assert.equal(changed.bounded, sync.bounded);
      assert.equal(changed.focusNodes, sync.focusNodes);
      assert.equal(changed.reason, sync.reason);
    } finally {
      changed.free();
      sync.free();
    }
  }
  const core = shaclValidateChangesToSarif(CORE_SHAPES, CONFORMING, added, removed);
  const sparql = shaclValidateChangesToSarif(SPARQL_SHAPES, CONFORMING, added, removed);
  assert.equal(core.bounded, true, "Core shapes bound the change");
  assert.equal(sparql.bounded, false, "SPARQL shapes fall back to the whole graph");
  assert.deepEqual(focusNodes(core.sarif), [`${EX}p1`, `${EX}p2`], "both halves of the change are validated");
  core.free();
  sparql.free();

  // Entailment.
  const entailed = shaclEntail(RULE_SHAPES, CONFORMING);
  assert.equal(await shaclEntailAsync(RULE_SHAPES, CONFORMING), entailed);
  assert.equal(entailed.split("\n").filter((line) => line.includes(`<${EX}adult>`)).length, 12);

  // Every product door, current product and bound identity alike.
  const product = shaclPackProduct(SPARQL_SHAPES);
  const identity = identityDigestOf(product);
  const reference = shaclValidateToSarif(SPARQL_SHAPES, NONCONFORMING);
  assert.equal(shaclProductValidateToSarif(product, NONCONFORMING), reference);
  assert.equal(await shaclProductValidateToSarifAsync(product, NONCONFORMING), reference);
  assert.equal(await shaclProductValidateToSarifRebuildAsync(product, NONCONFORMING), shaclProductValidateToSarifRebuild(product, NONCONFORMING));
  assert.equal(
    await shaclProductValidateToSarifExpectingAsync(product, NONCONFORMING, identity),
    shaclProductValidateToSarifExpecting(product, NONCONFORMING, identity),
  );
  assert.equal(
    await shaclProductValidateToSarifRebuildExpectingAsync(product, NONCONFORMING, identity),
    shaclProductValidateToSarifRebuildExpecting(product, NONCONFORMING, identity),
  );
  assert.equal(focusNodes(reference).length, 4);
});

test("a refused product rejects with the synchronous twin's ShaclProductRefusal, dimension and all", async () => {
  const product = shaclPackProduct(CORE_SHAPES);
  const corrupted = Uint8Array.from(product);
  corrupted[Math.floor(corrupted.length / 2)] ^= 0xff;

  const sync = syncThrow(() => shaclProductValidateToSarif(corrupted, NONCONFORMING));
  const refused = await rejection(shaclProductValidateToSarifAsync(corrupted, NONCONFORMING));
  assert.ok(refused instanceof ShaclProductRefusal, "the rejection is the structured refusal class");
  assert.equal(refused.dimension, "section-digest");
  assert.equal(refused.dimension, sync.dimension);
  assert.equal(refused.message, sync.message);
  assert.equal(typeof refused.evidence.async.polls, "number");
  refused.free();
  sync.free();

  // A product bound to a different identity: `shapes-graph`, on both lanes.
  const other = identityDigestOf(shaclPackProduct(SPARQL_SHAPES));
  const wrong = await rejection(shaclProductValidateToSarifExpectingAsync(product, NONCONFORMING, other));
  assert.ok(wrong instanceof ShaclProductRefusal);
  assert.equal(wrong.dimension, "shapes-graph");
  const wrongRebuild = await rejection(shaclProductValidateToSarifRebuildExpectingAsync(product, NONCONFORMING, other));
  assert.equal(wrongRebuild.dimension, "shapes-graph");
  // A selector that is not a digest: no dimension, because no product was inspected.
  const malformed = await rejection(shaclProductValidateToSarifExpectingAsync(product, NONCONFORMING, "not-hex"));
  assert.ok(malformed instanceof ShaclProductRefusal);
  assert.equal(malformed.dimension, undefined);
  assert.equal(malformed.message, syncThrow(() => shaclProductValidateToSarifExpecting(product, NONCONFORMING, "not-hex")).message);
  // A malformed data graph: a refusal with no dimension, as synchronously.
  const badData = await rejection(shaclProductValidateToSarifAsync(product, "<not n-triples"));
  assert.ok(badData instanceof ShaclProductRefusal);
  assert.equal(badData.dimension, undefined);

  // The valid neighbour: the unmodified product under its own identity resolves.
  assert.equal(
    await shaclProductValidateToSarifExpectingAsync(product, NONCONFORMING, identityDigestOf(product)),
    shaclValidateToSarif(CORE_SHAPES, NONCONFORMING),
  );
});

test("a SERVICE target: the synchronous twin refuses it, the asynchronous twin answers through the host and reports the remote answer", async () => {
  const shapes = serviceShapes(false);

  // The offline lane's refusal, by name.
  const refused = syncThrow(() => shaclValidateToSarif(shapes, ALICE_AND_BOB));
  assert.match(refused.message, /no remote query source configured/);
  // And the asynchronous twin with no handler refuses it the same way.
  const unhandled = await rejection(shaclValidateToSarifAsync(shapes, ALICE_AND_BOB));
  assert.match(unhandled.message, /no remote query source configured/);

  // Two registries, two answers, two different reports — each naming exactly the person
  // the registry banned.
  const alice = recordingResolver(async () => registryAnswer("alice", "carol"));
  const aliceReport = await shaclValidateToSarifAsync(shapes, ALICE_AND_BOB, null, { resolveService: alice.resolveService });
  assert.deepEqual(focusNodes(aliceReport), [`${EX}alice`]);
  assert.equal(alice.calls.length, 1, "one target query, one remote request");
  const [{ request, ctx }] = alice.calls;
  assert.equal(request.kind, "service");
  assert.equal(request.endpoint, ENDPOINT);
  assert.match(request.queryText, /^SELECT/);
  assert.match(request.queryText, new RegExp(`<${EX}status>`));
  assert.match(request.queryText, /"banned"/);
  assert.equal(ctx.silent, false);

  const bob = recordingResolver(async () => registryAnswer("bob"));
  const bobReport = await shaclValidateToSarifAsync(shapes, ALICE_AND_BOB, null, { resolveService: bob.resolveService });
  assert.deepEqual(focusNodes(bobReport), [`${EX}bob`]);
  assert.equal(bob.calls[0].request.queryText, request.queryText, "the same forwarded text");

  const nobody = await shaclValidateToSarifAsync(shapes, ALICE_AND_BOB, null, { resolveService: async () => registryAnswer() });
  assert.equal(resultCount(nobody), 0, "a registry banning nobody here: the data conforms");

  // An endpoint served in process answers identically, with no host call.
  const registry = Dataset.parse(`<${EX}bob> <${EX}status> "banned" .\n`, "nquads");
  try {
    assert.equal(
      await shaclValidateToSarifAsync(shapes, ALICE_AND_BOB, null, { localServices: { [ENDPOINT]: registry } }),
      bobReport,
    );
  } finally {
    registry.free();
  }

  // The change and entailment twins reach the host too.
  const changed = await shaclValidateChangesToSarifAsync(shapes, ALICE_AND_BOB, null, null, null, {
    resolveService: async () => registryAnswer("alice"),
  });
  try {
    assert.equal(changed.bounded, false, "a SPARQL target has no bounded footprint");
    assert.deepEqual(focusNodes(changed.sarif), [`${EX}alice`]);
  } finally {
    changed.free();
  }
});

test("SERVICE SILENT in a target with a failing resolver is the join identity, exactly as on queryAsync", async () => {
  const shapes = serviceShapes(true);
  const failing = () => recordingResolver(async () => ({ kind: "transport", message: "registry unreachable" }));

  const shaclMock = failing();
  const report = await shaclValidateToSarifAsync(shapes, ALICE_AND_BOB, null, { resolveService: shaclMock.resolveService });
  assert.equal(shaclMock.calls.length, 1, "SILENT still asked the endpoint");
  assert.equal(shaclMock.calls[0].ctx.silent, true);

  // queryAsync's answer to the target query under the same failure: the identity, which
  // joined with the local people selects every one of them.
  const queryMock = failing();
  const data = Dataset.parse(ALICE_AND_BOB, "nquads");
  try {
    const result = await new QueryEngine().queryAsync(data, TARGET_QUERY, { resolveService: queryMock.resolveService });
    const targeted = result.rows
      .toArray()
      .map((row) => row.this.value)
      .sort();
    assert.deepEqual(targeted, [`${EX}alice`, `${EX}bob`]);
    assert.deepEqual(focusNodes(report), targeted, "the report's focus nodes are queryAsync's rows");
  } finally {
    data.free();
  }
  // The synchronous twin has no source at all: no endpoint was reached, so it refuses the
  // SILENT target rather than answering the identity.
  assert.match(
    syncThrow(() => shaclValidateToSarif(shapes, ALICE_AND_BOB)).message,
    /no remote query source configured.*SILENT does not apply/,
  );

  // The answered neighbour differs: a registry banning Bob targets Bob alone.
  const answered = await shaclValidateToSarifAsync(shapes, ALICE_AND_BOB, null, { resolveService: async () => registryAnswer("bob") });
  assert.deepEqual(focusNodes(answered), [`${EX}bob`]);

  // The non-SILENT neighbour of the same failure rejects, with the transport's words.
  const loud = await rejection(
    shaclValidateToSarifAsync(serviceShapes(false), ALICE_AND_BOB, null, { resolveService: failing().resolveService }),
  );
  assert.match(loud.message, /SERVICE <http:\/\/example\.org\/sparql>: transport: registry unreachable/);
});

// Large enough that the synchronous validation plainly occupies the event loop.
const HEAVY = people(20_000, { badAge: (i) => i % 7 === 0 });

test("a heavy validation with no SPARQL in it yields to the event loop; the synchronous twin does not", async () => {
  let ticks = 0;
  const interval = setInterval(() => {
    ticks += 1;
  }, 5);
  try {
    const sync = shaclValidateToSarif(CORE_SHAPES, HEAVY);
    const syncTicks = ticks;
    ticks = 0;
    const report = await shaclValidateToSarifAsync(CORE_SHAPES, HEAVY, null, { yieldEveryPolls: 64 });
    const asyncTicks = ticks;
    assert.equal(syncTicks, 0, "the synchronous twin never turns the event loop");
    assert.ok(asyncTicks > 0, `the interval ticked ${asyncTicks} times during the asynchronous validation`);
    assert.equal(report, sync, "yielding changed nothing reported");
    assert.equal(resultCount(report), Math.ceil(20_000 / 7));
  } finally {
    clearInterval(interval);
  }
});

test("a signal stops a SHACL validation: its reason, a timeout, and an already-aborted signal", async () => {
  // Aborted mid-validation, between focus nodes of a validation with no SPARQL in it.
  const controller = new AbortController();
  const reason = new Error("the caller gave up");
  setTimeout(() => controller.abort(reason), 1);
  const aborted = await rejection(
    shaclValidateToSarifAsync(CORE_SHAPES, HEAVY, null, { signal: controller.signal, yieldEveryPolls: 16 }),
  );
  assert.equal(aborted, reason, "the twin rejects with the signal's own reason");

  // The entailment twin stops between a rule's focus nodes the same way.
  const entailController = new AbortController();
  setTimeout(() => entailController.abort(reason), 1);
  assert.equal(
    await rejection(shaclEntailAsync(RULE_SHAPES, HEAVY, null, { signal: entailController.signal, yieldEveryPolls: 16 })),
    reason,
  );

  // A timeout signal rejects with its TimeoutError, as on every ungoverned twin.
  const deadline = AbortSignal.timeout(1);
  const timedOut = await rejection(
    shaclValidateToSarifAsync(CORE_SHAPES, HEAVY, null, { signal: deadline, yieldEveryPolls: 16 }),
  );
  assert.equal(timedOut.name, "TimeoutError");
  assert.equal(timedOut, deadline.reason);

  // An already-aborted signal rejects before any job begins: the resolver is never asked.
  const early = new AbortController();
  early.abort(reason);
  const never = recordingResolver(async () => registryAnswer("alice"));
  assert.equal(
    await rejection(shaclValidateToSarifAsync(serviceShapes(false), ALICE_AND_BOB, null, { signal: early.signal, resolveService: never.resolveService })),
    reason,
  );
  assert.equal(
    await rejection(shaclProductValidateToSarifAsync(shaclPackProduct(CORE_SHAPES), CONFORMING, { signal: early.signal })),
    reason,
  );
  assert.equal(never.calls.length, 0, "no job ran, so no SERVICE was asked");

  // A ceiling is refused by name — no synchronous SHACL entry takes one — and so is a
  // SPARQL operation's `base`.
  const ceiling = await rejection(shaclValidateToSarifAsync(CORE_SHAPES, CONFORMING, null, { deadlineMs: 20 }));
  assert.ok(ceiling instanceof TypeError);
  assert.match(ceiling.message, /deadlineMs is an execution governor/);
  const base = await rejection(shaclValidateToSarifAsync(CORE_SHAPES, CONFORMING, null, { base: EX }));
  assert.ok(base instanceof TypeError);
  assert.match(base.message, /unknown query option "base"/);

  // The neighbour: a signal that never fires resolves to the synchronous twin's report.
  assert.equal(
    await shaclValidateToSarifAsync(CORE_SHAPES, NONCONFORMING, null, { signal: new AbortController().signal, yieldEveryPolls: 0 }),
    shaclValidateToSarif(CORE_SHAPES, NONCONFORMING),
  );
  assert.equal(stackPointer(), IDLE);
});

// Two different validations suspended at once, at every poll, with synchronous
// validations run on the main stack while both wait. The SHACL engine keeps its
// governors, SERVICE sources, registries and call depth in per-thread scopes installed
// by RAII guards; guards assume they nest, and suspended jobs do not. Each job's scopes
// must travel with it: a synchronous validation must see none of them (it must still
// refuse a SERVICE for want of a source while a job holding one is suspended, and must
// not poll a suspended job's signal), and each job must resume under its own.
test("concurrent SHACL validations suspended at every poll, interleaved with synchronous validations, all answer their synchronous baselines", async () => {
  const sparqlData = people(40, { shortNick: (i) => i % 5 === 0 });
  const serviceData = `${ALICE_AND_BOB}<${EX}dave> <${RDF_TYPE}> <${EX}Person> .\n`;
  const baseline = {
    sparql: shaclValidateToSarif(SPARQL_SHAPES, sparqlData),
    core: shaclValidateToSarif(CORE_SHAPES, NONCONFORMING),
    entail: shaclEntail(RULE_SHAPES, CONFORMING),
  };
  assert.equal(focusNodes(baseline.sparql).length, 8);
  assert.equal(focusNodes(baseline.core).length, 3);

  const yieldEvery = { yieldEveryPolls: 0 };
  let settled = 0;
  const track = (job) => {
    job.then(
      () => (settled += 1),
      () => (settled += 1),
    );
    return job;
  };
  let turns = 0;
  const turn = async () => {
    assert.equal(shaclValidateToSarif(CORE_SHAPES, NONCONFORMING), baseline.core, `sync core, turn ${turns}`);
    assert.equal(shaclValidateToSarif(SPARQL_SHAPES, sparqlData), baseline.sparql, `sync sparql, turn ${turns}`);
    // A suspended job's SERVICE source is its own: the synchronous lane still has none.
    assert.match(
      syncThrow(() => shaclValidateToSarif(serviceShapes(false), serviceData)).message,
      /no remote query source configured/,
      `sync service, turn ${turns}`,
    );
    assert.equal(stackPointer(), IDLE);
    turns += 1;
    await new Promise((resolve) => setImmediate(resolve));
  };

  const jobs = { sparql: track(shaclValidateToSarifAsync(SPARQL_SHAPES, sparqlData, null, yieldEvery)) };
  await turn();
  await turn();
  jobs.service = track(
    shaclValidateToSarifAsync(serviceShapes(false), serviceData, null, {
      ...yieldEvery,
      resolveService: async () => registryAnswer("bob", "dave"),
    }),
  );
  jobs.entail = track(shaclEntailAsync(RULE_SHAPES, CONFORMING, null, yieldEvery));
  while (settled < Object.keys(jobs).length) await turn();
  assert.ok(turns > 10, `the jobs interleaved over ${turns} turns`);

  assert.equal(await jobs.sparql, baseline.sparql, "the SHACL-SPARQL job's report");
  assert.deepEqual(focusNodes(await jobs.service), [`${EX}bob`, `${EX}dave`], "the SERVICE job's report");
  assert.equal(await jobs.entail, baseline.entail, "the entailment job's graph");

  // Afterwards every lane still answers exactly: nothing a job installed outlives it.
  assert.equal(shaclValidateToSarif(SPARQL_SHAPES, sparqlData), baseline.sparql);
  assert.match(syncThrow(() => shaclValidateToSarif(serviceShapes(false), serviceData)).message, /no remote query source configured/);
  assert.equal(await shaclValidateToSarifAsync(CORE_SHAPES, NONCONFORMING, null, yieldEvery), baseline.core);
  assert.equal(stackPointer(), IDLE);
});
