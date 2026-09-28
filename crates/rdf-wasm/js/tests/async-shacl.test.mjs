// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution tests for the asynchronous SHACL twins — `shaclValidateToSarifAsync`,
// `shaclValidateChangesToSarifAsync`, `shaclEntailAsync`, `shaclApplyRulesAsync`,
// `shaclEvalNodeExprAsync` and the four `shaclProductValidateToSarif…Async` — driven
// through the package root against the actual optimized wasm module under WebAssembly
// JavaScript Promise Integration.
//
// SHACL evaluates SPARQL (`sh:SPARQLTarget` queries, SHACL-SPARQL constraints, SHACL-AF
// rules and node expressions), so each twin runs its synchronous twin's own body as a
// job: the job yields and stops on its signal, and the signal is polled between focus
// nodes too. SHACL-SPARQL admits no `SERVICE` in any query, so a shapes graph with one is
// refused while it loads, on either lane, and no host handler is ever asked. Every oracle
// compares an asynchronous answer with an independent observation — the synchronous
// twin's value — and every fixture pair is chosen so the compared values differ from case
// to case, so an equality cannot be satisfied by a twin that returns a constant.
//
// Wall-clock bounds are never asserted: the machine running this is not quiet.

import { test } from "node:test";
import assert from "node:assert/strict";

import {
  ready,
  shaclEntail,
  shaclEntailAsync,
  shaclPackProduct,
  shaclProductExplain,
  ShaclImportError,
  ShaclProductRefusal,
  shaclApplyRules,
  shaclApplyRulesAsync,
  shaclEvalNodeExpr,
  shaclEvalNodeExprAsync,
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
const stackPointer = () => exports.purrdf_stack_pointer.value >>> 0;
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
// constraint every person violates (none has a clearance). SHACL-SPARQL admits no
// SERVICE in any query, a target's included, so this shapes graph is refused while it
// loads.
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
const SERVICE_REFUSAL = /a federated query \(SERVICE\) is not allowed/;

// The neighbour of `serviceShapes`: the same target reading the data graph itself.
const LOCAL_TARGET_SHAPES = `${PREFIXES}
ex:BannedShape a sh:NodeShape ;
  sh:target [
    a sh:SPARQLTarget ;
    sh:select """
      SELECT ?this WHERE { ?this a <${EX}Person> ; <${EX}status> "banned" }
    """ ;
  ] ;
  sh:property [ sh:path ex:clearance ; sh:minCount 1 ] .
`;

// A node expression walking `ex:age` from its focus node, for the node-expression twin.
const PATH_EXPRESSION_SHAPES = `${PREFIXES}
ex:AgeOf sh:path ex:age .
`;

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

/** `ALICE_AND_BOB`, with each of `who` banned. */
const banning = (...who) =>
  `${ALICE_AND_BOB}${who.map((name) => `<${EX}${name}> <${EX}status> "banned" .\n`).join("")}`;

/** `shaclValidateToSarifAsync` over the shapes, data and `shapesBase`, with host `options`. */
const validateAsync = (shapes, data, shapesBase, options) =>
  shaclValidateToSarifAsync(shapes, data, shapesBase, undefined, undefined, undefined, undefined, undefined, options);

/** `shaclEntailAsync` over the shapes, data and `shapesBase`, with host `options`. */
const entailAsync = (shapes, data, shapesBase, options) =>
  shaclEntailAsync(shapes, data, shapesBase, undefined, undefined, undefined, undefined, undefined, undefined, undefined, options);

/** A `ShaclEntailment`'s two facts, the entailment freed. */
/** A `ShaclDiagnostic` array as plain `{ rule, shape }` values, comparable across calls. */
function diagnosticFacts(diagnostics) {
  return diagnostics.map((diagnostic) => ({ rule: diagnostic.rule, shape: diagnostic.shape }));
}

function entailmentFacts(entailment) {
  try {
    return { ntriples: entailment.ntriples, diagnostics: diagnosticFacts(entailment.diagnostics) };
  } finally {
    entailment.free();
  }
}

/** A `ShaclNodeExprOutcome`'s output nodes and diagnostics, freeing it. */
function nodeExprFacts(outcome) {
  try {
    return { outputs: outcome.outputs, diagnostics: diagnosticFacts(outcome.diagnostics) };
  } finally {
    outcome.free();
  }
}

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
    assert.equal(await validateAsync(shapes, data, null, { yieldEveryPolls: 0 }), sync, label);
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
  const entailed = entailmentFacts(shaclEntail(RULE_SHAPES, CONFORMING));
  assert.deepEqual(entailmentFacts(await shaclEntailAsync(RULE_SHAPES, CONFORMING)), entailed);
  assert.equal(entailed.ntriples.split("\n").filter((line) => line.includes(`<${EX}adult>`)).length, 12);

  // Rules: the inference graph, its proof and its diagnostics.
  const ruleFacts = (inference) => {
    try {
      return {
        inferred: inference.inferred,
        proof: inference.proof,
        diagnostics: diagnosticFacts(inference.diagnostics),
      };
    } finally {
      inference.free();
    }
  };
  const applied = ruleFacts(shaclApplyRules(CONFORMING, RULE_SHAPES, undefined, undefined, undefined, true));
  assert.equal(applied.inferred.split("\n").filter((line) => line.includes(`<${EX}adult>`)).length, 12);
  assert.deepEqual(
    ruleFacts(await shaclApplyRulesAsync(CONFORMING, RULE_SHAPES, undefined, undefined, undefined, true)),
    applied,
  );
  assert.notEqual(
    ruleFacts(await shaclApplyRulesAsync(NONCONFORMING, RULE_SHAPES)).inferred,
    ruleFacts(await shaclApplyRulesAsync(people(3), RULE_SHAPES)).inferred,
  );

  // A node expression: the same output nodes, which differ from focus node to focus node.
  for (const focus of [`${EX}p1`, `${EX}p2`]) {
    const nodes = nodeExprFacts(shaclEvalNodeExpr(PATH_EXPRESSION_SHAPES, CONFORMING, `${EX}AgeOf`, focus));
    assert.equal(nodes.outputs.length, 1, focus);
    assert.deepEqual(
      nodeExprFacts(await shaclEvalNodeExprAsync(PATH_EXPRESSION_SHAPES, CONFORMING, `${EX}AgeOf`, focus)),
      nodes,
    );
  }
  assert.notDeepEqual(
    nodeExprFacts(await shaclEvalNodeExprAsync(PATH_EXPRESSION_SHAPES, CONFORMING, `${EX}AgeOf`, `${EX}p1`)).outputs,
    nodeExprFacts(await shaclEvalNodeExprAsync(PATH_EXPRESSION_SHAPES, CONFORMING, `${EX}AgeOf`, `${EX}p2`)).outputs,
  );

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

test("a SERVICE in a SHACL-SPARQL target is refused on both lanes and no host handler is asked; the local target answers", async () => {
  const resolver = recordingResolver(async () => {
    throw new Error("a refused shapes graph asks no endpoint");
  });
  for (const silent of [false, true]) {
    const shapes = serviceShapes(silent);
    const refused = syncThrow(() => shaclValidateToSarif(shapes, ALICE_AND_BOB));
    assert.match(refused.message, SERVICE_REFUSAL);
    for (const options of [undefined, { resolveService: resolver.resolveService }]) {
      const rejected = await rejection(validateAsync(shapes, ALICE_AND_BOB, null, options));
      assert.equal(rejected.message, refused.message);
      assert.equal(typeof rejected.evidence.async.polls, "number");
    }
    const changed = await rejection(
      shaclValidateChangesToSarifAsync(shapes, ALICE_AND_BOB, null, null, null, undefined, undefined, undefined, {
        resolveService: resolver.resolveService,
      }),
    );
    assert.match(changed.message, SERVICE_REFUSAL);
  }
  assert.equal(resolver.calls.length, 0, "no endpoint was asked");

  // The neighbour: the same target over the data graph is admitted, and each report
  // names exactly the people the data bans.
  for (const who of [["alice"], ["bob"], []]) {
    const data = banning(...who);
    const sync = shaclValidateToSarif(LOCAL_TARGET_SHAPES, data);
    assert.deepEqual(focusNodes(sync), who.map((name) => `${EX}${name}`).sort());
    assert.equal(await validateAsync(LOCAL_TARGET_SHAPES, data), sync);
  }
});

test("the shapes-graph arguments reach the job exactly as they reach the synchronous twin", async () => {
  // An unresolved `owl:imports` rejects with the same `ShaclImportError`.
  const importing = `${PREFIXES}@prefix owl: <http://www.w3.org/2002/07/owl#> .
<${EX}shapes> a owl:Ontology ; owl:imports <${EX}imported> .
${CORE_SHAPES}`;
  const sync = syncThrow(() => shaclValidateToSarif(importing, NONCONFORMING));
  assert.ok(sync instanceof ShaclImportError);
  const refused = await rejection(shaclValidateToSarifAsync(importing, NONCONFORMING));
  assert.ok(refused instanceof ShaclImportError, "the rejection is the structured import class");
  assert.equal(refused.kind, sync.kind);
  assert.deepEqual(refused.iris, sync.iris);
  assert.equal(refused.message, sync.message);
  assert.equal(typeof refused.evidence.async.polls, "number");
  refused.free();
  sync.free();

  // The resolved neighbour: the import table supplied, both lanes validate.
  const importIris = [`${EX}imported`];
  const importDocuments = [`<${EX}imported> a <http://www.w3.org/2002/07/owl#Ontology> .\n`];
  assert.equal(
    await shaclValidateToSarifAsync(importing, NONCONFORMING, undefined, undefined, importIris, importDocuments),
    shaclValidateToSarif(importing, NONCONFORMING, undefined, undefined, importIris, importDocuments),
  );

  // A conformance-disallow set changes the verdict the log records, on both lanes.
  const disallows = ["http://www.w3.org/ns/shacl#Violation"];
  const judged = shaclValidateToSarif(CORE_SHAPES, NONCONFORMING, undefined, disallows);
  assert.equal(await shaclValidateToSarifAsync(CORE_SHAPES, NONCONFORMING, undefined, disallows), judged);
  const empty = syncThrow(() => shaclValidateToSarif(CORE_SHAPES, NONCONFORMING, undefined, []));
  assert.equal((await rejection(shaclValidateToSarifAsync(CORE_SHAPES, NONCONFORMING, undefined, []))).message, empty.message);

  // `shapesGraph` names the graph a SPARQL rule's `$shapesGraph` is pre-bound to: named
  // and omitted, the two entailments differ, and each job's is its synchronous twin's.
  const shapesGraphRule = `${PREFIXES}
ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
  sh:rule [ a sh:SPARQLRule ; sh:construct """
    CONSTRUCT { $this <${EX}shapesGraph> ?g }
    WHERE { BIND (COALESCE($shapesGraph, <${EX}none>) AS ?g) }""" ] .
`;
  const named = entailmentFacts(
    shaclEntail(shapesGraphRule, ALICE_AND_BOB, undefined, undefined, undefined, `${EX}shapes-graph`),
  );
  const unnamed = entailmentFacts(shaclEntail(shapesGraphRule, ALICE_AND_BOB));
  assert.match(named.ntriples, new RegExp(`<${EX}shapes-graph>`));
  assert.notEqual(named.ntriples, unnamed.ntriples);
  assert.deepEqual(
    entailmentFacts(await shaclEntailAsync(shapesGraphRule, ALICE_AND_BOB, undefined, undefined, undefined, `${EX}shapes-graph`)),
    named,
  );
  assert.deepEqual(entailmentFacts(await shaclEntailAsync(shapesGraphRule, ALICE_AND_BOB)), unnamed);
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
    const report = await validateAsync(CORE_SHAPES, HEAVY, null, { yieldEveryPolls: 64 });
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
    validateAsync(CORE_SHAPES, HEAVY, null, { signal: controller.signal, yieldEveryPolls: 16 }),
  );
  assert.equal(aborted, reason, "the twin rejects with the signal's own reason");

  // The entailment twin stops between a rule's focus nodes the same way.
  const entailController = new AbortController();
  setTimeout(() => entailController.abort(reason), 1);
  assert.equal(
    await rejection(entailAsync(RULE_SHAPES, HEAVY, null, { signal: entailController.signal, yieldEveryPolls: 16 })),
    reason,
  );

  // A timeout signal rejects with its TimeoutError, as on every ungoverned twin.
  const deadline = AbortSignal.timeout(1);
  const timedOut = await rejection(
    validateAsync(CORE_SHAPES, HEAVY, null, { signal: deadline, yieldEveryPolls: 16 }),
  );
  assert.equal(timedOut.name, "TimeoutError");
  assert.equal(timedOut, deadline.reason);

  // An already-aborted signal rejects before any job begins: the resolver is never asked.
  const early = new AbortController();
  early.abort(reason);
  const never = recordingResolver(async () => {
    throw new Error("no job ran");
  });
  assert.equal(
    await rejection(validateAsync(LOCAL_TARGET_SHAPES, banning("alice"), null, { signal: early.signal, resolveService: never.resolveService })),
    reason,
  );
  assert.equal(
    await rejection(shaclProductValidateToSarifAsync(shaclPackProduct(CORE_SHAPES), CONFORMING, { signal: early.signal })),
    reason,
  );
  assert.equal(never.calls.length, 0, "no job ran, so no handler was asked");

  // A ceiling is refused by name — no synchronous SHACL entry takes one — and so is a
  // SPARQL operation's `base`.
  const ceiling = await rejection(validateAsync(CORE_SHAPES, CONFORMING, null, { deadlineMs: 20 }));
  assert.ok(ceiling instanceof TypeError);
  assert.match(ceiling.message, /deadlineMs is an execution governor/);
  const base = await rejection(validateAsync(CORE_SHAPES, CONFORMING, null, { base: EX }));
  assert.ok(base instanceof TypeError);
  assert.match(base.message, /unknown query option "base"/);

  // The neighbour: a signal that never fires resolves to the synchronous twin's report.
  assert.equal(
    await validateAsync(CORE_SHAPES, NONCONFORMING, null, { signal: new AbortController().signal, yieldEveryPolls: 0 }),
    shaclValidateToSarif(CORE_SHAPES, NONCONFORMING),
  );
  assert.equal(stackPointer(), IDLE);
});

// Different validations suspended at once, at every poll, with synchronous validations
// run on the main stack while they wait. The SHACL engine keeps its governors, sources,
// registries and call depth in per-thread scopes installed by RAII guards; guards assume
// they nest, and suspended jobs do not. Each job's scopes must travel with it: a
// synchronous validation must see none of them (it must not poll a suspended job's
// signal), and each job must resume under its own.
test("concurrent SHACL validations suspended at every poll, interleaved with synchronous validations, all answer their synchronous baselines", async () => {
  const sparqlData = people(40, { shortNick: (i) => i % 5 === 0 });
  const targetData = `${banning("bob", "dave")}<${EX}dave> <${RDF_TYPE}> <${EX}Person> .\n`;
  const baseline = {
    sparql: shaclValidateToSarif(SPARQL_SHAPES, sparqlData),
    core: shaclValidateToSarif(CORE_SHAPES, NONCONFORMING),
    target: shaclValidateToSarif(LOCAL_TARGET_SHAPES, targetData),
    entail: entailmentFacts(shaclEntail(RULE_SHAPES, CONFORMING)),
  };
  assert.equal(focusNodes(baseline.sparql).length, 8);
  assert.equal(focusNodes(baseline.core).length, 3);
  assert.deepEqual(focusNodes(baseline.target), [`${EX}bob`, `${EX}dave`]);

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
    assert.equal(shaclValidateToSarif(LOCAL_TARGET_SHAPES, targetData), baseline.target, `sync target, turn ${turns}`);
    assert.equal(stackPointer(), IDLE);
    turns += 1;
    await new Promise((resolve) => setTimeout(resolve, 0));
  };

  const jobs = { sparql: track(validateAsync(SPARQL_SHAPES, sparqlData, null, yieldEvery)) };
  await turn();
  await turn();
  jobs.target = track(validateAsync(LOCAL_TARGET_SHAPES, targetData, null, yieldEvery));
  jobs.entail = track(entailAsync(RULE_SHAPES, CONFORMING, null, yieldEvery));
  while (settled < Object.keys(jobs).length) await turn();
  assert.ok(turns > 10, `the jobs interleaved over ${turns} turns`);

  assert.equal(await jobs.sparql, baseline.sparql, "the SHACL-SPARQL job's report");
  assert.equal(await jobs.target, baseline.target, "the SPARQL-target job's report");
  assert.deepEqual(entailmentFacts(await jobs.entail), baseline.entail, "the entailment job's graph");

  // Afterwards every lane still answers exactly: nothing a job installed outlives it.
  assert.equal(shaclValidateToSarif(SPARQL_SHAPES, sparqlData), baseline.sparql);
  assert.equal(await validateAsync(CORE_SHAPES, NONCONFORMING, null, yieldEvery), baseline.core);
  assert.equal(stackPointer(), IDLE);
});
