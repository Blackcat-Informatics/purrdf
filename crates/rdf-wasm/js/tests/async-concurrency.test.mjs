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
  // polling, so an operator chain run beneath a few EXISTS levels (which poll) goes far
  // past a 512 KiB region's base before the next poll — into the overrun zone beneath it,
  // never into another allocation. The request is the deepest the parser admits of its
  // shape: sixteen EXISTS levels around a chain of 477 additions, one short of the
  // expression-height budget. Measured on the shipped artifact by painting the region:
  // it reaches about 631 000 bytes below the region's top, about 107 000 past a 512 KiB
  // region's base, while its deepest poll is about 271 000 bytes down — so the canary,
  // not the guard band, is what stops it. The synchronous lane runs it on its own 1 MiB
  // stack (about 638 000 bytes deep). The job fails with the same typed error, and the
  // instance stays intact.
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
  assert.equal(overran.message, "asynchronous job stack region exhausted (524288 bytes); raise stackBytes");
  const overranPollDepth = overran.evidence.async.stackHighWaterBytes;
  assert.ok(
    overranPollDepth < SMALL_REGION - GUARD_BAND,
    `no poll reached the guard band (${overranPollDepth} bytes deep): the canary stopped the job`,
  );
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

test("a trap poisons the instance with a clear error", () => {
  const child = spawnSync(
    process.execPath,
    ["--wasm-stack-switching-stack-size=32", fileURLToPath(new URL("./fixtures/trap-child.mjs", import.meta.url))],
    { encoding: "utf8", timeout: 120_000 },
  );
  assert.equal(child.status, 0, `the child exited ${child.status}: ${child.stderr}`);
  const report = JSON.parse(child.stdout.trim());
  const POISON =
    "the wasm instance trapped (RangeError: Maximum call stack size exceeded) and must be re-instantiated";

  assert.deepEqual(report.asyncBefore, { settled: "resolved", rowCount: 1 });
  // The valid neighbour: synchronously, the same query is the parser's own typed refusal
  // — twice, so it is an error and not a trap that poisoned anything.
  for (const sync of [report.syncDeep, report.syncDeepAgain]) {
    assert.equal(sync.settled, "rejected");
    assert.match(sync.message, /group graph pattern nesting exceeds the safety limit of 128/);
  }
  assert.deepEqual(report.trapped, { settled: "rejected", name: "Error", message: POISON });
  assert.deepEqual(report.inFlight, { settled: "rejected", name: "Error", message: POISON });
  assert.deepEqual(report.asyncAfter, { settled: "rejected", name: "Error", message: POISON });
  assert.deepEqual(report.updateAfter, { settled: "rejected", name: "Error", message: POISON });
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
