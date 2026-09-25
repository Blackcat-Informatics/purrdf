// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution tests for CONCURRENT asynchronous jobs on one wasm instance: jobs
// suspended at once and resumed out of order, synchronous calls running while a job's
// frames sit in its private stack region, a job started from inside a wasm→JS callback,
// stack-region exhaustion, trap poisoning, and admission.
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
import init from "../pkg/purrdf_wasm.js";

await ready();
// Already instantiated: `init` hands back the one instance's raw exports.
const exports = await init();
const stackPointer = () => exports.__wbindgen_add_to_stack_pointer(0) >>> 0;
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
    await new Promise((resolve) => setImmediate(resolve));
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
    await new Promise((resolve) => setImmediate(resolve));
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
  const suspended = engine.queryAsync(local(), JOIN, { resolveService: host.resolveService, stackBytes: 524288 });
  await turnUntil(() => host.asked === 1, "the small-region job suspending");
  // 64 nested OPTIONALs, synchronously, while the job's frames sit in its 512 KiB region.
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
    await new Promise((resolve) => setImmediate(resolve));
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

// Measured on the shipped artifact through `evidence.async.stackHighWaterBytes` (the
// deepest poll below the region's top), with the chain above: 64 nested OPTIONALs reach
// 319 037 bytes, 100 reach 487 805. A 512 KiB region stops a poll inside its 128 KiB
// guard band, so it can host at most 524 288 − 131 072 = 393 216 bytes of polling
// frames: 64 fits, 100 does not.
const EXHAUSTING_DEPTH = 100;
const SMALL_REGION = 524288;
const GUARD_BAND = 128 * 1024;
/** The evaluator's own stack refusal, as a job's error carries it. */
const EVALUATION_STACK_REFUSAL = /native-sparql-evaluation-stack-exhausted.*evaluation stack exhausted/;

test("stack region exhaustion is a typed error, not corruption", async () => {
  const engine = new QueryEngine();
  const chain = Dataset.parse(CHAIN, "nquads");
  const query = nestedOptional(EXHAUSTING_DEPTH);

  let exhausted;
  try {
    await engine.queryGovernedAsync(chain, query, { stackBytes: SMALL_REGION });
  } catch (error) {
    exhausted = error;
  }
  assert.ok(exhausted instanceof Error, "the small region refuses the query");
  assert.equal(
    exhausted.message,
    "asynchronous job stack region exhausted (524288 bytes); raise stackBytes",
  );
  const exhaustedDepth = exhausted.evidence.async.stackHighWaterBytes;
  assert.ok(
    exhaustedDepth > SMALL_REGION - GUARD_BAND - 16 && exhaustedDepth < SMALL_REGION,
    `the job stopped inside the guard band (${exhaustedDepth} bytes deep)`,
  );
  assert.equal(stackPointer(), IDLE);

  // The instance is intact: the synchronous answer is unchanged, and an ordinary
  // asynchronous query still answers.
  assert.equal(engine.select(chain, nestedOptional(64)).rowCount, 200);
  assert.equal((await engine.queryAsync(chain, nestedOptional(8))).rowCount, 200);

  // Frames that never poll: expression evaluation recurses once per operator without
  // polling, so an operator chain run beneath a few EXISTS levels (which poll) would go
  // far past a 512 KiB region's base before the next poll. The request is the deepest the
  // parser admits of its shape: sixteen EXISTS levels around a chain of 477 additions,
  // one short of the expression-height budget. Measured on the shipped artifact by
  // painting the region, it needs about 631 000 bytes below the region's top — about
  // 107 000 past a 512 KiB region's base — while its deepest poll is about 271 000 bytes
  // down, so the guard band never sees it. What stops it is the evaluator's own stack
  // guard, which checks every operator against the stack left above the region's base
  // (the job installs that base as its stack floor) and refuses with 64 KiB still to
  // spare. The overrun zone beneath the base is therefore never touched: had any frame
  // reached it, the run's zone inspection would have latched the region's exhaustion,
  // which outranks every other error, so the evaluator's refusal arriving as the job's
  // error is the proof. The synchronous lane runs the chain on its own 1 MiB stack (about
  // 638 000 bytes deep), and a 4 MiB region answers it.
  const deepChain = `SELECT ?s WHERE { ?s <${EX}p> ?o ${`FILTER EXISTS { ?s <${EX}p> ?o `.repeat(16)}BIND(1 AS ?one) FILTER(${"?one + ".repeat(477)}?one > 0)${" }".repeat(16)} }`;
  // A separate engine answers it synchronously: the engine caches a parsed plan per
  // query text, and a cached plan would spare the asynchronous run its parse.
  assert.equal(new QueryEngine().select(chain, deepChain).rowCount, 200, "the synchronous lane answers it");
  let overran;
  try {
    await engine.queryGovernedAsync(chain, deepChain, { stackBytes: SMALL_REGION });
  } catch (error) {
    overran = error;
  }
  assert.ok(overran instanceof Error, "the small region refuses the deep chain");
  assert.match(overran.message, EVALUATION_STACK_REFUSAL);
  assert.match(overran.message, /expression was reached with less than 65536 bytes of stack left/);
  const overranPollDepth = overran.evidence.async.stackHighWaterBytes;
  assert.ok(
    overranPollDepth < SMALL_REGION - GUARD_BAND,
    `no poll reached the guard band (${overranPollDepth} bytes deep): the evaluator's guard stopped the job`,
  );
  assert.equal(stackPointer(), IDLE);
  // The same refusal when the job gives the event loop back at every poll: the region's
  // floor is put back on every resumption, so the guard still measures against the
  // region and not against the synchronous stack's floor — which, far below every heap
  // region, would leave the chain unguarded until it reached the overrun zone.
  let yielding;
  try {
    await engine.queryGovernedAsync(chain, deepChain, { stackBytes: SMALL_REGION, yieldEveryPolls: 0 });
  } catch (error) {
    yielding = error;
  }
  assert.ok(yielding instanceof Error, "the small region refuses the deep chain between yields too");
  assert.match(yielding.message, EVALUATION_STACK_REFUSAL);
  assert.ok(yielding.evidence.async.yields > 16, `${yielding.evidence.async.yields} yields`);
  assert.equal(stackPointer(), IDLE);
  assert.equal(engine.select(chain, nestedOptional(64)).rowCount, 200);
  assert.equal((await engine.queryAsync(chain, nestedOptional(8))).rowCount, 200);
  const chainOnLargeRegion = await new QueryEngine().queryGovernedAsync(chain, deepChain, {
    stackBytes: 4 * 1024 * 1024,
  });
  assert.equal(chainOnLargeRegion.isComplete, true, "the 4 MiB neighbour evaluates and answers it");
  assert.equal(chainOnLargeRegion.result.rowCount, 200);

  // The neighbour: the same query on a 4 MiB region answers — deeper than the small
  // region could ever have hosted.
  const answered = await engine.queryGovernedAsync(chain, query, { stackBytes: 4 * 1024 * 1024 });
  assert.equal(answered.isComplete, true);
  assert.equal(answered.result.rowCount, 200);
  const answeredDepth = answered.evidence.async.stackHighWaterBytes;
  assert.ok(answeredDepth > SMALL_REGION - GUARD_BAND, `${answeredDepth} bytes deep`);
  assert.ok(answeredDepth < 4 * 1024 * 1024 - GUARD_BAND);
  assert.equal(stackPointer(), IDLE);
});

// Nesting the parser admits can need more stack than a region holds: 63 nested
// `FILTER NOT EXISTS` or 126 nested `LATERAL` trapped the synchronous lane's 1 MiB stack
// before the evaluator guarded its own recursion. On the smallest region both are the
// region's typed exhaustion — their frames poll at every algebra node, so the guard band
// stops them before the evaluator's own check would — and the instance is not poisoned;
// on a 16 MiB region both answer, with the rows their semantics give.
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

test("admitted nesting too deep for the smallest region is a typed error there, the instance is not poisoned, and a 16 MiB region answers it", async () => {
  const engine = new QueryEngine();
  const data = Dataset.parse(NEST_DATA, "nquads");
  const before = data.canonicalize();
  for (const [what, query, expected] of [
    // An odd number of negations of "`?s` has a `<q>`": every subject but `s1`.
    ["63 nested FILTER NOT EXISTS", nestedAround(`?s <${EX}p> ?o FILTER NOT EXISTS { `, 63), ["s2", "s3", "s4"]],
    // Only `s1` has a `<q>`, at every level.
    ["126 nested LATERAL", nestedAround(`?s <${EX}p> ?o LATERAL { `, 126), ["s1"]],
  ]) {
    // Twice: a trap or an overrun of the region's zone would poison the instance, and the
    // second job would reject with the poison instead of the same typed error.
    for (let attempt = 0; attempt < 2; attempt += 1) {
      await assert.rejects(
        engine.queryAsync(data, query, { stackBytes: SMALL_REGION }),
        { message: "asynchronous job stack region exhausted (524288 bytes); raise stackBytes" },
        `${what} on the smallest region`,
      );
      assert.equal(stackPointer(), IDLE);
    }
    // Not poisoned: both lanes answer exactly, over a dataset whose canonical form is
    // byte-identical to what it was before.
    assert.equal(data.canonicalize(), before);
    assert.deepEqual(subjectsOf(engine.select(data, `SELECT ?s WHERE { ?s <${EX}q> ?o }`)), ["s1"]);
    assert.deepEqual(
      subjectsOf(await engine.queryAsync(data, `SELECT ?s WHERE { ?s <${EX}q> ?o }`, { stackBytes: SMALL_REGION })),
      ["s1"],
    );
    // The valid neighbour: a 16 MiB region evaluates it.
    const answered = await engine.queryAsync(data, query, { stackBytes: 16 * 1024 * 1024 });
    assert.deepEqual(subjectsOf(answered), expected, `${what} on a 16 MiB region`);
    assert.equal(stackPointer(), IDLE);
  }
});

// Nesting is bounded by the parser, not by the region. A FILTER nested 10 000
// parentheses deep would recurse through the parser without a poll, far past any
// region; the parser refuses the level past its limit with a typed syntax error first.
const NESTING_LIMIT = 128;
const PARENTHESIZED_REFUSAL = new RegExp(
  `bracketted expression nesting exceeds the safety limit of ${NESTING_LIMIT}`,
);
/** A FILTER comparing `?o` with itself, the left operand wrapped in `depth` parentheses. */
const parenthesizedFilter = (depth) =>
  `SELECT ?s WHERE { ?s <${EX}p> ?o FILTER(${"(".repeat(depth)}?o${")".repeat(depth)} = ?o) }`;

test("a FILTER nested 10 000 parentheses deep is a typed parse error on the smallest region, and the instance is not poisoned", async () => {
  const engine = new QueryEngine();
  const chain = Dataset.parse(CHAIN, "nquads");
  // Twice: a trap or an overrun of the region's zone would poison the instance, and the
  // second job would reject with the poison instead of the parser's refusal.
  for (let attempt = 0; attempt < 2; attempt += 1) {
    await assert.rejects(
      engine.queryAsync(chain, parenthesizedFilter(10_000), { stackBytes: SMALL_REGION }),
      PARENTHESIZED_REFUSAL,
    );
    assert.equal(stackPointer(), IDLE);
  }
  // The instance is intact on both lanes.
  assert.equal((await engine.queryAsync(chain, nestedOptional(8), { stackBytes: SMALL_REGION })).rowCount, 200);
  assert.equal(engine.select(chain, nestedOptional(8)).rowCount, 200);
  assert.equal(stackPointer(), IDLE);
});

test("the deepest parenthesised FILTER the parser admits answers on the smallest region", async () => {
  const engine = new QueryEngine();
  const chain = Dataset.parse(CHAIN, "nquads");
  // The WHERE group is the first nesting level, so a FILTER inside it holds one
  // parenthesis fewer than the limit. `?o = ?o` holds on every chain edge.
  const deepest = await engine.queryAsync(chain, parenthesizedFilter(NESTING_LIMIT - 1), {
    stackBytes: SMALL_REGION,
  });
  assert.equal(deepest.rowCount, 200, "every chain edge passes the deepest admitted FILTER");
  // The refused neighbour, one parenthesis deeper.
  await assert.rejects(
    engine.queryAsync(chain, parenthesizedFilter(NESTING_LIMIT), { stackBytes: SMALL_REGION }),
    PARENTHESIZED_REFUSAL,
  );
  assert.equal(stackPointer(), IDLE);
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
  const THREW = { settled: "threw", name: "Error", message: POISON };

  // The valid neighbours, on the very objects the trap later poisons: a job that
  // finishes, one that faults and one whose host reports a typed failure each settle as
  // their own job's answer or error, and afterwards every lane answers exactly — the
  // committed update included — on those objects and on new ones.
  assert.deepEqual(report.asyncBefore, { settled: "resolved", subjects: [`${EX}a`] });
  assert.deepEqual(report.faulted, {
    settled: "rejected",
    name: "Error",
    message: "resolveService rejected: Error: the example.org endpoint refused",
  });
  assert.deepEqual(report.typedFailure, {
    settled: "rejected",
    name: "Error",
    message:
      "error native-sparql-query-eval: SERVICE federation error: SERVICE <http://example.org/sparql>: " +
      "transport: the example.org endpoint is unreachable",
  });
  assert.deepEqual(report.updateBefore, { settled: "resolved" });
  assert.deepEqual(report.syncBefore, { settled: "resolved", subjects: [`${EX}a`, `${EX}b`] });
  assert.deepEqual(report.asyncAfterFaults, { settled: "resolved", subjects: [`${EX}a`, `${EX}b`] });
  assert.deepEqual(report.newEngineBefore, { settled: "resolved", subjects: [`${EX}c`] });
  assert.deepEqual(report.sizeBefore, { settled: "returned", value: 2 });
  // Synchronously, the query that traps asynchronously is the parser's own typed refusal
  // — twice, so it is an error and not a trap that poisoned anything, and the
  // synchronous lane's stack is its own after the jobs above.
  for (const sync of [report.syncDeep, report.syncDeepAgain]) {
    assert.equal(sync.settled, "rejected");
    assert.match(sync.message, /group graph pattern nesting exceeds the safety limit of 128/);
  }

  // The trap: the job and the one in flight reject with the poison, and so does every
  // later call — asynchronous, synchronous, on objects created before the trap, new
  // objects, statics, free functions and `ready()` itself.
  for (const name of ["trapped", "inFlight", "asyncAfter", "updateAfter", "readyAfter"]) {
    assert.deepEqual(report[name], REJECTED, name);
  }
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
    assert.deepEqual(report[name], THREW, name);
  }
  // Releasing is the one call that does not throw: it releases nothing.
  assert.deepEqual(report.freeAfter, { settled: "returned" });

  // The enumerated surface: every entry refuses with the poison, and the enumeration
  // reached every function the package root exports, so it cannot pass by being empty.
  const { surface } = report;
  for (const [name, outcome] of Object.entries(surface)) {
    assert.deepEqual(outcome, REJECTED, name);
  }
  for (const [name, value] of Object.entries(packageRoot)) {
    if (typeof value !== "function") continue;
    const source = Function.prototype.toString.call(value);
    if (!/^class\b/.test(source)) assert.ok(name in surface, `${name} was not called`);
    else if (/\n\s*constructor\(/.test(source)) assert.ok(`new ${name}` in surface, `new ${name} was not called`);
  }
  for (const name of ["new QueryEngine", "new Dataset", "Dataset.parse", "version", "ready", "dataset.size", "engine.query()"]) {
    assert.ok(name in surface, `${name} is missing from the enumerated surface`);
  }
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
