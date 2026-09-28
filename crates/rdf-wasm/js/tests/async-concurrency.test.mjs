// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution tests for CONCURRENT asynchronous jobs on one wasm instance: jobs
// suspended at once and resumed out of order, synchronous calls running while a job's
// frames sit in its private stack region, a job started from inside a wasm→JS callback,
// stack exhaustion on a job's region, trap poisoning, and admission.
//
// The shadow-stack pointer is read straight from the instance's exports after every
// scenario: a pointer left inside a job's region while JavaScript runs is the defect the
// scheduler's switching rules exist to prevent, and it would corrupt a suspended job's
// frames silently. Every scenario therefore ends by checking it is back at its idle value.

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import * as packageRoot from "../index.mjs";
import { Dataset, QueryEngine, configureAsync, ready } from "../index.mjs";
import init, { asyncStackRegionBytes } from "../pkg/purrdf_wasm.js";
import { HOST_STACK_REFUSAL, NESTING_SHAPES, NUMBERS, attempt, realEnd } from "./fixtures/nesting.mjs";
import { assertSurfacePoisoned, threwAtGate } from "./fixtures/poisoned-surface.mjs";

await ready();
// Already instantiated: `init` hands back the one instance's raw exports.
const exports = await init();
const stackPointer = () => exports.purrdf_stack_pointer.value >>> 0;
const IDLE = stackPointer();

const EX = "http://example.org/";
const LOCAL_NT = [`<${EX}a> <${EX}p> <${EX}o1> .`, `<${EX}b> <${EX}p> <${EX}o2> .`, ""].join("\n");
const local = () => Dataset.parse(LOCAL_NT, "nquads");
const JOIN = `SELECT ?s ?x WHERE { ?s <${EX}p> ?o . SERVICE <${EX}sparql> { ?o <${EX}q> ?x } }`;

/** The remote answer binding each local object to `tag`-suffixed values. */
const remote = (tag) =>
  JSON.stringify({
    head: { vars: ["o", "x"] },
    results: {
      bindings: [
        { o: { type: "uri", value: `${EX}o1` }, x: { type: "literal", value: `${tag}1` } },
        { o: { type: "uri", value: `${EX}o2` }, x: { type: "literal", value: `${tag}2` } },
      ],
    },
  });
const joined = (tag) => [`s=${EX}a&x=${tag}1`, `s=${EX}b&x=${tag}2`];

function rowsOf(select) {
  return select.rows
    .toArray()
    .map((row) =>
      Object.keys(row)
        .sort()
        .map((name) => `${name}=${row[name].value}`)
        .join("&"),
    )
    .sort();
}

async function turnUntil(predicate, label) {
  for (let turns = 0; turns < 10_000; turns += 1) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  assert.fail(`the event loop turned 10 000 times without ${label}`);
}

/** A resolver the test answers by hand; `asked` counts its calls. */
function deferred() {
  const state = { asked: 0, release: undefined };
  state.resolveService = () =>
    new Promise((resolve) => {
      state.asked += 1;
      state.release = resolve;
    });
  return state;
}

// A chain `n0 → n1 → … → n200` and a query nesting one OPTIONAL per level: each level's
// frames are live while the next is evaluated, so its stack depth grows with the
// nesting while its answer stays one row per chain node.
const CHAIN = (() => {
  const lines = [];
  for (let index = 0; index < 200; index += 1) lines.push(`<${EX}n${index}> <${EX}p> <${EX}n${index + 1}> .`);
  return `${lines.join("\n")}\n`;
})();
function nestedOptional(depth) {
  let pattern = `?v${depth} <${EX}p> ?v${depth + 1}`;
  for (let level = depth - 1; level >= 0; level -= 1) {
    pattern = `?v${level} <${EX}p> ?v${level + 1} OPTIONAL { ${pattern} }`;
  }
  return `SELECT * WHERE { ${pattern} }`;
}

test("three interleaved async queries resume out of order and all answer correctly", async () => {
  const engine = new QueryEngine();
  const data = local();
  const hosts = { A: deferred(), B: deferred(), C: deferred() };
  const runs = Object.fromEntries(
    Object.entries(hosts).map(([tag, host]) => [
      tag,
      engine.queryAsync(data, JOIN, { resolveService: host.resolveService, yieldEveryPolls: 0 }),
    ]),
  );
  await turnUntil(() => Object.values(hosts).every((host) => host.asked === 1), "all three jobs suspending");
  assert.equal(stackPointer(), IDLE, "three suspended jobs leave the pointer at its idle value");
  const syncControl = `SELECT ?s ?o WHERE { ?s <${EX}p> ?o }`;
  for (const tag of ["C", "A", "B"]) {
    hosts[tag].release(remote(tag));
    // A synchronous query between resumptions runs on the main stack, beside the
    // suspended jobs' regions.
    assert.equal(engine.select(data, syncControl).rowCount, 2);
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  for (const [tag, run] of Object.entries(runs)) {
    assert.deepEqual(rowsOf(await run), joined(tag), `job ${tag} joined its own answer`);
  }
  assert.equal(stackPointer(), IDLE);
});

test("a deep sync query during a suspension is answered", async () => {
  const engine = new QueryEngine();
  const chain = Dataset.parse(CHAIN, "nquads");
  const expected = engine.select(chain, nestedOptional(64)).rowCount;
  assert.equal(expected, 200);

  const host = deferred();
  const suspended = engine.queryAsync(local(), JOIN, { resolveService: host.resolveService });
  await turnUntil(() => host.asked === 1, "the job suspending");
  // 64 nested OPTIONALs, synchronously, while the job's frames sit in its region.
  const deep = engine.select(chain, nestedOptional(64));
  assert.equal(deep.rowCount, expected);
  const first = deep.rows.take(0);
  assert.equal(first.v0.value, `${EX}n0`);
  assert.equal(first.v64.value, `${EX}n64`);
  host.release(remote("deep"));
  assert.deepEqual(rowsOf(await suspended), joined("deep"), "the suspended job's frames survived");
  assert.equal(stackPointer(), IDLE);
});

// A property path's traversal runs inside the evaluator's walk scope — the scope that
// latches a stack refusal for the traversal and discards what it built — and polls the
// job's stop signal at every step, so under `yieldEveryPolls: 0` a job suspends with that
// scope open. Whatever runs while it waits (another job, a synchronous query) opens and
// closes scopes of its own, and the jobs close theirs in whatever order the event loop
// resumes them, which no single stack would: the scope state is part of each context the
// scheduler switches between, beside its stack floor.
const PATH_CHAIN = (() => {
  const lines = [];
  for (let index = 0; index < 12; index += 1) lines.push(`<${EX}n${index}> <${EX}p> <${EX}n${index + 1}> .`);
  return `${lines.join("\n")}\n`;
})();
const PATH_SELECT = `SELECT ?s ?o WHERE { ?s <${EX}p>+ ?o }`;
const PATH_BACKWARD = `SELECT ?s WHERE { ?s <${EX}p>/<${EX}p>* <${EX}n12> }`;
const PATH_CONSTRUCT = `CONSTRUCT { ?o <${EX}reachedFrom> ?s } WHERE { ?s <${EX}p>* ?o }`;

test("jobs suspended inside a property path's walk scope, interleaved with each other and with synchronous paths, all answer their synchronous baselines", async () => {
  const engine = new QueryEngine();
  const data = Dataset.parse(PATH_CHAIN, "nquads");
  // Baselines from a separate engine, before any job has run on the instance.
  const baselineEngine = new QueryEngine();
  const baseline = {
    select: rowsOf(baselineEngine.select(data, PATH_SELECT)),
    backward: rowsOf(baselineEngine.select(data, PATH_BACKWARD)),
    construct: baselineEngine.construct(data, PATH_CONSTRUCT).canonicalize(),
  };
  // 12 edges: 78 pairs one or more steps apart, 12 subjects reaching n12, and 91
  // reflexive-or-longer pairs to construct.
  assert.equal(baseline.select.length, 78);
  assert.equal(baseline.backward.length, 12);
  assert.equal(engine.construct(data, PATH_CONSTRUCT).size, 91);

  const yieldEvery = { yieldEveryPolls: 0 };
  let settled = 0;
  const track = (job) => {
    job.then(() => (settled += 1), () => (settled += 1));
    return job;
  };
  let turns = 0;
  /** Synchronous path queries, on the main stack, while every unfinished job sits
   * suspended — mostly mid-traversal, with its walk scope open — then one turn. */
  const turn = async () => {
    assert.deepEqual(rowsOf(engine.select(data, PATH_SELECT)), baseline.select, `sync select, turn ${turns}`);
    assert.equal(engine.construct(data, PATH_CONSTRUCT).canonicalize(), baseline.construct, `sync construct, turn ${turns}`);
    assert.equal(stackPointer(), IDLE);
    turns += 1;
    await new Promise((resolve) => setTimeout(resolve, 0));
  };
  // The short job opens its scope first and closes it while the two started after it
  // still have theirs open: the reverse of the order a single stack would close them in.
  const jobs = { backward: track(engine.queryGovernedAsync(data, PATH_BACKWARD, yieldEvery)) };
  await turn();
  await turn();
  jobs.select = track(engine.queryGovernedAsync(data, PATH_SELECT, yieldEvery));
  jobs.construct = track(engine.constructAsync(data, PATH_CONSTRUCT, yieldEvery));
  while (settled < Object.keys(jobs).length) await turn();
  assert.ok(turns > 10, `the jobs interleaved over ${turns} turns`);

  const select = await jobs.select;
  assert.equal(select.isComplete, true);
  assert.deepEqual(rowsOf(select.result), baseline.select, "the select job's answer");
  assert.ok(select.evidence.async.yields > 10, `${select.evidence.async.yields} yields`);
  const backward = await jobs.backward;
  assert.equal(backward.isComplete, true);
  assert.deepEqual(rowsOf(backward.result), baseline.backward, "the backward job's answer");
  assert.ok(backward.evidence.async.yields > 10, `${backward.evidence.async.yields} yields`);
  assert.equal((await jobs.construct).canonicalize(), baseline.construct, "the construct job's graph");

  // Afterwards every lane still answers exactly — nothing a job left behind (a scope
  // it never closed, a refusal it latched) outlives it.
  assert.deepEqual(rowsOf(engine.select(data, PATH_SELECT)), baseline.select);
  assert.equal(engine.construct(data, PATH_CONSTRUCT).canonicalize(), baseline.construct);
  assert.deepEqual(rowsOf(await engine.queryAsync(data, PATH_BACKWARD, yieldEvery)), baseline.backward);
  assert.equal(stackPointer(), IDLE);
});

test("an async job started from inside a sync callback runs and returns", async () => {
  const engine = new QueryEngine();
  const data = local();
  const host = deferred();
  const chunks = [];
  let started;
  let pointerInsideCallback;
  data.serializeToSink("nquads", undefined, {
    write(chunk) {
      if (started === undefined) {
        // Inside a wasm→JS call: the main stack pointer is below its idle value here.
        pointerInsideCallback = stackPointer();
        started = engine.queryAsync(data, JOIN, { resolveService: host.resolveService, yieldEveryPolls: 0 });
      }
      chunks.push(chunk.slice());
    },
  });
  assert.ok(pointerInsideCallback < IDLE, "the job really was started inside a wasm call");
  assert.equal(stackPointer(), IDLE, "the serialization returned with the pointer restored");
  const serialized = new TextDecoder().decode(Buffer.concat(chunks));
  assert.equal(serialized, data.serialize("nquads"), "the callback's wasm call finished intact");
  await turnUntil(() => host.asked === 1, "the job suspending on its SERVICE");
  host.release(remote("cb"));
  assert.deepEqual(rowsOf(await started), joined("cb"));
  assert.equal(stackPointer(), IDLE);
});

// Every job runs on a stack region exactly as large as the module's own shadow stack, so
// the two lanes run out of stack at the same depth: a request the synchronous lane
// evaluates a job evaluates too, and a request too deep for the one is too deep for the
// other, refused with the evaluator's own words. Measured on the shipped artifact through
// `evidence.async.stackHighWaterBytes` (the deepest poll below the region's top): 100
// nested OPTIONALs reach about 410 000 bytes and 110 about 450 000, so 140 reach past
// 393 216 bytes and still answer on the region, as they do on the synchronous lane.
const DEEP_OPTIONAL = 140;
/** The size of every job's stack region, as the module reports it. */
const REGION_BYTES = asyncStackRegionBytes();
/** The evaluator's own stack refusal, as a job's error carries it. */
const EVALUATION_STACK_REFUSAL = /native-sparql-evaluation-stack-exhausted.*evaluation stack exhausted/;
/**
 * The region's own faults, which only frames no check guards may reach: a poll outside
 * the region, or the canary word at its base overwritten.
 */
const REGION_FAULT = /outside its stack region|ran past the base of its stack region/;

test("a request as deep as the synchronous lane evaluates answers on a job's region, frames that never poll included", async () => {
  // The shipped module is linked stack-first: its shadow stack is `[0, idle pointer)`, and
  // a job's region is exactly that large.
  assert.equal(REGION_BYTES, IDLE, "the region is the size of the synchronous lane's stack");
  const engine = new QueryEngine();
  const chain = Dataset.parse(CHAIN, "nquads");
  const query = nestedOptional(DEEP_OPTIONAL);
  assert.equal(engine.select(chain, query).rowCount, 200, "the synchronous lane answers it");

  // Twice: a trap or a region fault would poison the instance or fail the job, and the
  // second job would reject instead of answering the same.
  for (let attempt = 0; attempt < 2; attempt += 1) {
    const answered = await engine.queryGovernedAsync(chain, query);
    assert.equal(answered.isComplete, true);
    assert.equal(answered.result.rowCount, 200);
    const depth = answered.evidence.async.stackHighWaterBytes;
    assert.ok(depth > 393_216 && depth < REGION_BYTES, `${depth} bytes deep in a ${REGION_BYTES}-byte region`);
    assert.equal(stackPointer(), IDLE);
  }
  // The shallow neighbour answers every row too.
  assert.equal((await engine.queryAsync(chain, nestedOptional(8))).rowCount, 200);

  // Frames that never poll: expression evaluation recurses once per level of the
  // expression tree without polling, so an expression nested deeply enough beneath a few
  // EXISTS levels (which poll) stands far below the deepest poll. A flat operator chain
  // is no such tree — its operands are one node, folded by a loop — so the depth is
  // written as nesting: each bracket level spells `(?one = 2 || ?one = 1 && (…) != false)`,
  // five tree levels (`||`, `&&`, and the two `!=` builds) above the bracket inside it.
  // Every level passes its inner level's truth through (`false || (true && x != false)`
  // is `x`), so the answer is decided by the innermost comparison: the whole tree is
  // evaluated, and an innermost `?one = 2` answers no row. The request is the deepest the
  // parser admits of its shape: sixteen EXISTS levels around 94 bracket levels, one short
  // of the bracket-nesting budget, a tree 470 levels tall inside the expression-height
  // budget. Measured on the shipped artifact: the synchronous lane runs it about 545 000
  // bytes deep, while its deepest poll is about 253 500 bytes down. On a job's region the
  // tree's frames stand between polls far below anything a poll records, inside the
  // region; the canary at the region's base is read when the run returns, so a frame past
  // the base would have failed the job with the region's fault instead of answering.
  const deepTree = (innermost) => {
    let expression = innermost;
    for (let level = 0; level < 94; level += 1) {
      expression = `(?one = 2 || ?one = 1 && ${expression} != false)`;
    }
    return `SELECT ?s WHERE { ?s <${EX}p> ?o ${`FILTER EXISTS { ?s <${EX}p> ?o `.repeat(16)}BIND(1 AS ?one) FILTER(${expression})${" }".repeat(16)} }`;
  };
  const deepChain = deepTree("(?one = 1)");
  // The neighbour that observes the evaluation: the same tree with a false innermost
  // comparison answers nothing.
  const deepFalse = deepTree("(?one = 2)");
  // Separate engines throughout: the engine caches a parsed plan per query text, and a
  // cached plan would spare a later run its parse.
  assert.equal(new QueryEngine().select(chain, deepFalse).rowCount, 0, "the innermost comparison decides");
  assert.equal(new QueryEngine().select(chain, deepChain).rowCount, 200, "the synchronous lane answers it");
  const tree = await new QueryEngine().queryGovernedAsync(chain, deepChain);
  assert.equal(tree.isComplete, true, "the job's region hosts the whole tree");
  assert.equal(tree.result.rowCount, 200);
  assert.ok(
    tree.evidence.async.stackHighWaterBytes < REGION_BYTES / 2,
    `the deepest poll (${tree.evidence.async.stackHighWaterBytes} bytes) stands far above the tree's frames`,
  );
  const treeFalse = await new QueryEngine().queryGovernedAsync(chain, deepFalse);
  assert.equal(treeFalse.isComplete, true);
  assert.equal(treeFalse.result.rowCount, 0, "its false neighbour answers nothing on the region too");
  assert.equal(stackPointer(), IDLE);
  // The same answer when the job gives the event loop back at every poll: the region's
  // floor is put back on every resumption, so every measurement is against the region.
  const yielding = await new QueryEngine().queryGovernedAsync(chain, deepChain, { yieldEveryPolls: 0 });
  assert.equal(yielding.isComplete, true);
  assert.equal(yielding.result.rowCount, 200);
  assert.ok(yielding.evidence.async.yields > 16, `${yielding.evidence.async.yields} yields`);
  assert.equal(stackPointer(), IDLE);
  assert.equal(engine.select(chain, nestedOptional(64)).rowCount, 200);
});

// Nesting the parser admits can need more stack than a lane holds. 126 nested `LATERAL`
// cost the evaluator about 9 KB of shadow stack a level, past the synchronous lane's
// 1 MiB, and a job's region is exactly as large: on both lanes it is the evaluator's own
// typed refusal, word for word the same, and the instance is not poisoned. 63 nested
// `FILTER NOT EXISTS` — a nested `EXISTS` body is substituted when it is evaluated — and
// 40 nested `LATERAL` are the valid neighbours: both lanes answer them with the rows
// their semantics give.
const NEST_DATA = [1, 2, 3, 4]
  .map((n) => `<${EX}s${n}> <${EX}p> <${EX}o${n}> .`)
  .concat([`<${EX}s1> <${EX}q> <${EX}o1> .`])
  .join("\n");
const nestedAround = (open, depth) =>
  `SELECT ?s WHERE { ${open.repeat(depth)}?s <${EX}q> ?z${" }".repeat(depth)} }`;
const subjectsOf = (result) =>
  result.rows
    .toArray()
    .map((row) => row.s.value.replace(EX, ""))
    .sort();

test("admitted nesting answers or is refused on a job's region exactly as on the synchronous lane, and the instance is not poisoned", async () => {
  const engine = new QueryEngine();
  const data = Dataset.parse(NEST_DATA, "nquads");
  const before = data.canonicalize();
  const lateral = (depth) => nestedAround(`?s <${EX}p> ?o LATERAL { `, depth);

  // The refusal: the synchronous lane's message, and the job's, twice — a trap or a
  // region fault would poison the instance or fail the job, and the second job would
  // reject with something else.
  let syncRefusal;
  assert.throws(() => engine.select(data, lateral(126)), (error) => {
    syncRefusal = error.message;
    return true;
  });
  assert.match(syncRefusal, /^error native-sparql-evaluation-stack-exhausted: evaluation stack exhausted: /);
  for (let attempt = 0; attempt < 2; attempt += 1) {
    await assert.rejects(
      engine.selectAsync(data, lateral(126)),
      (error) => {
        assert.equal(error.message, syncRefusal, "the job's refusal is the synchronous lane's, word for word");
        assert.doesNotMatch(error.message, REGION_FAULT);
        return true;
      },
      "126 nested LATERAL on a job's region",
    );
    assert.equal(stackPointer(), IDLE);
  }
  // A governed job reports the same refusal with its evidence: its polls stayed inside
  // the region, and the evaluator stopped it.
  let governed;
  try {
    await engine.queryGovernedAsync(data, lateral(126));
  } catch (error) {
    governed = error;
  }
  assert.ok(governed instanceof Error, "the governed twin refuses it too");
  assert.match(governed.message, EVALUATION_STACK_REFUSAL);
  const depth = governed.evidence.async.stackHighWaterBytes;
  assert.ok(depth > 0 && depth < REGION_BYTES, `${depth} bytes deep in a ${REGION_BYTES}-byte region`);
  assert.equal(stackPointer(), IDLE);

  // The valid neighbours answer on both lanes, with the rows their semantics give.
  for (const [what, query, expected] of [
    // Only `s1` has a `<q>`, at every level.
    ["40 nested LATERAL", lateral(40), ["s1"]],
    // An odd number of negations of "`?s` has a `<q>`": every subject but `s1`.
    ["63 nested FILTER NOT EXISTS", nestedAround(`?s <${EX}p> ?o FILTER NOT EXISTS { `, 63), ["s2", "s3", "s4"]],
  ]) {
    assert.deepEqual(subjectsOf(engine.select(data, query)), expected, `${what}, synchronously`);
    assert.deepEqual(subjectsOf(await engine.selectAsync(data, query)), expected, `${what} on a job's region`);
    assert.equal(stackPointer(), IDLE);
  }
  // Not poisoned: both lanes answer exactly, over a dataset whose canonical form is
  // byte-identical to what it was before.
  assert.equal(data.canonicalize(), before);
  const plain = `SELECT ?s WHERE { ?s <${EX}q> ?o }`;
  assert.deepEqual(subjectsOf(engine.select(data, plain)), ["s1"]);
  assert.deepEqual(subjectsOf(await engine.selectAsync(data, plain)), ["s1"]);
});

// Brackets build no node: a FILTER nested 10 000 parentheses deep parses to the same
// expression as one pair, keeps its nesting in the parser's heap stacks rather than on
// either stack a job runs on, and so answers on a job's region and on the synchronous
// lane — with exactly the rows one pair answers. Each job leaves the stack pointer idle
// and the instance unpoisoned.
const parenthesizedFilter = (depth) =>
  `SELECT ?s WHERE { ?s <${EX}p> ?o FILTER(${"(".repeat(depth)}?o${")".repeat(depth)} = ?o) }`;

test("a FILTER nested 10 000 parentheses deep answers what one pair answers, on both lanes", async () => {
  const engine = new QueryEngine();
  const chain = Dataset.parse(CHAIN, "nquads");
  const expected = engine.select(chain, parenthesizedFilter(1)).rowCount;
  assert.ok(expected > 0, "the shallow neighbour answers rows");
  assert.equal(engine.select(chain, parenthesizedFilter(10_000)).rowCount, expected);
  assert.equal(stackPointer(), IDLE);
  // Twice: a trap or a region fault would poison the instance or fail the job, and the
  // second job would reject instead of answering.
  for (let attempt = 0; attempt < 2; attempt += 1) {
    const answer = await engine.queryAsync(chain, parenthesizedFilter(10_000));
    assert.equal(answer.rowCount, expected, `attempt ${attempt + 1}`);
    assert.equal(stackPointer(), IDLE);
  }
  // The instance is intact on both lanes.
  assert.equal((await engine.queryAsync(chain, nestedOptional(8))).rowCount, 200);
  assert.equal(engine.select(chain, nestedOptional(8)).rowCount, 200);
  assert.equal(stackPointer(), IDLE);
});

// Every shape at 128 levels answers on a job's region with the value its nesting
// computes. Brackets, property-path groups and a group holding only a group build no node
// (see `query.test.mjs`), so those shapes answer 20 000 levels deep on a job's region too.
// Calls and negations build a node per level, and the real end of each is found on the
// region by bisection — the deepest level that answers, and one level more the host-stack
// refusal. Measured on this build, and bounded by the plan-height admission's
// `WASM_VALUE_LIMIT` (2 304 expression nodes), which is the same on both lanes: 2 302
// calls and 2 303 negations answer, as on the synchronous lane, and the job's refusal is
// the synchronous lane's word for word. Each run is a fresh engine, so no cached plan
// spares a job its parse.
test("nesting answers on the asynchronous lane as deep as the job's stacks hold it, and is the synchronous lane's refusal past that", async () => {
  const data = Dataset.parse(NUMBERS, "nquads");
  const run = (query) => new QueryEngine().queryAsync(data, query);
  const runSync = (query) => new QueryEngine().select(data, query);
  const limits = {};
  for (const shape of NESTING_SHAPES) {
    const [what, text, expected] = shape;
    assert.deepEqual(
      (await attempt(run, text(128), `${what} on a job's region`)).subjects,
      expected,
      `${what} 128 deep answers on a job's region`,
    );
    const deep = await attempt(run, text(20_000), `${what} on a job's region`);
    if (deep.subjects) {
      assert.deepEqual(deep.subjects, expected, `${what}: 20 000 levels answer on a job's region`);
    } else {
      const { deepest, refusal } = await realEnd(run, shape);
      limits[what] = deepest;
      assert.match(refusal, HOST_STACK_REFUSAL, what);
      const { refusal: syncRefusal } = await realEnd(runSync, shape);
      assert.equal(refusal, syncRefusal, `${what}: the job's refusal is the synchronous lane's`);
    }
    assert.equal(stackPointer(), IDLE);
  }
  assert.deepEqual(limits, { "nested ABS(": 2302, "nested -(": 2303 });
  // Not poisoned: the synchronous lane answers after all of it.
  assert.equal(new QueryEngine().select(Dataset.parse(CHAIN, "nquads"), nestedOptional(8)).rowCount, 200);
});

test("a trap poisons every entry point of the instance, and jobs that fault without trapping poison nothing", () => {
  const child = spawnSync(
    process.execPath,
    ["--wasm-stack-switching-stack-size=32", fileURLToPath(new URL("./fixtures/trap-child.mjs", import.meta.url))],
    { encoding: "utf8", timeout: 120_000 },
  );
  assert.equal(child.status, 0, `the child exited ${child.status}: ${child.stderr}`);
  const report = JSON.parse(child.stdout.trim());
  const POISON =
    "the wasm instance trapped (RangeError: Maximum call stack size exceeded) and cannot be used again; " +
    "load the package in a fresh JavaScript realm (a new page, Worker isolate or process)";
  const REJECTED = { settled: "rejected", name: "Error", message: POISON };

  // The valid neighbours, on the very objects the trap later poisons: a job that
  // finishes, one whose host rejects (its invocation fails as the host's fault) and one
  // whose host reports a typed failure each settle as their own job's answer or error,
  // and afterwards every lane answers exactly — the committed update included — on those
  // objects and on new ones.
  assert.deepEqual(report.asyncBefore, { settled: "resolved", subjects: [`${EX}a`] });
  assert.equal(report.faulted.settled, "rejected");
  assert.equal(report.faulted.name, "Error");
  assert.equal(report.faultedCode, "native-sparql-host-fault");
  assert.match(report.faulted.message, /the example\.org endpoint refused/, "the handler's own words reach the host that wrote it");
  assert.deepEqual(report.typedFailure, {
    settled: "rejected",
    name: "Error",
    message:
      "error native-sparql-service-failed: SERVICE federation error: SERVICE <http://example.org/sparql>: " +
      "transport: the example.org endpoint is unreachable",
  });
  assert.deepEqual(report.updateBefore, { settled: "resolved" });
  assert.deepEqual(report.syncBefore, { settled: "resolved", subjects: [`${EX}a`, `${EX}b`] });
  assert.deepEqual(report.asyncAfterFaults, { settled: "resolved", subjects: [`${EX}a`, `${EX}b`] });
  assert.deepEqual(report.newEngineBefore, { settled: "resolved", subjects: [`${EX}c`] });
  assert.deepEqual(report.sizeBefore, { settled: "returned", value: 2 });
  // Synchronously, the query that traps asynchronously answers on the main stack —
  // twice, so the first run poisoned nothing, and the synchronous lane's stack is its own
  // after the jobs above.
  for (const sync of [report.syncDeep, report.syncDeepAgain]) {
    assert.deepEqual(sync, { settled: "resolved", subjects: [`${EX}a`, `${EX}b`] });
  }

  // The trap: the job and the one in flight reject with the poison, and so does every
  // later asynchronous call and `ready()` itself. The run's gate entry never returned, so
  // the gate's counters no longer balance and its flag is set.
  for (const name of ["trapped", "inFlight", "asyncAfter", "updateAfter", "readyAfter"]) {
    assert.deepEqual(report[name], REJECTED, name);
  }
  assert.deepEqual(report.gateAfter, { poisoned: 1, balanced: false });
  // Every synchronous entry — on objects created before the trap, new objects, statics,
  // free functions — traps at the gate linked into the module.
  for (const name of [
    "syncAfter",
    "syncDeepAfter",
    "sizeAfter",
    "addAfter",
    "iterateAfter",
    "quadAfter",
    "newEngineAfter",
    "newDatasetAfter",
    "parseAfter",
    "versionAfter",
  ]) {
    assert.deepEqual(report[name], threwAtGate, name);
  }
  // Releasing is the one entry that returns: the release exports' gate is inert on a
  // poisoned instance, so `free()` and `[Symbol.dispose]()` return without entering it.
  assert.deepEqual(report.freeAfter, { settled: "returned" });
  assert.ok(
    report.disposeAfter.settled === "returned" || report.disposeAfter.settled === "absent",
    `[Symbol.dispose]() on the poisoned instance: ${JSON.stringify(report.disposeAfter)}`,
  );

  // The enumerated surface: every entry refuses — with the poison or at the gate — and
  // the enumeration reached every function the package root exports, so it cannot pass
  // by being empty.
  assertSurfacePoisoned(report.surface, packageRoot, POISON);
});

test("beginAsync beyond maxConcurrentJobs is refused", async () => {
  const engine = new QueryEngine();
  const data = local();
  const admitted = async (limit) => {
    configureAsync({ maxConcurrentJobs: limit });
    const hosts = Array.from({ length: 16 }, () => deferred());
    const runs = hosts.map((host) => engine.queryAsync(data, JOIN, { resolveService: host.resolveService }));
    await turnUntil(() => hosts.every((host) => host.asked === 1), "sixteen jobs suspending");
    const extra = deferred();
    let refused;
    const run = engine.queryAsync(data, JOIN, { resolveService: extra.resolveService }).then(
      (answer) => ({ rows: rowsOf(answer) }),
      (error) => {
        refused = { error: error.message };
        return refused;
      },
    );
    await turnUntil(() => extra.asked === 1 || refused !== undefined, "the seventeenth job starting or refused");
    if (extra.asked === 1) extra.release(remote("extra"));
    const seventeenth = await run;
    hosts.forEach((host, index) => host.release(remote(`j${index}`)));
    const answers = await Promise.all(runs);
    answers.forEach((answer, index) => assert.deepEqual(rowsOf(answer), joined(`j${index}`)));
    return seventeenth;
  };
  try {
    assert.deepEqual(await admitted(16), {
      error:
        "too many asynchronous jobs in flight (the limit is 16); await one, or raise the limit with configureAsync({ maxConcurrentJobs })",
    });
    // The neighbour: a limit of 17 admits the seventeenth job, which answers.
    assert.deepEqual(await admitted(17), { rows: joined("extra") });
  } finally {
    configureAsync({ maxConcurrentJobs: 16 });
  }
  assert.throws(() => configureAsync({ maxConcurrentJobs: 0 }), RangeError);
  assert.equal(stackPointer(), IDLE);
});
