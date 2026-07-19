// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Node-side parse throughput benchmark for the purrdf wasm engine.
//
// Report-only (never a gate). It drives the ACTUAL compiled wasm through the
// public `Dataset.parse` surface over a large, deterministically generated
// N-Triples corpus. Parsing is the primary beneficiary of WebAssembly SIMD:
// the engine's byte scanning runs through `memchr`, whose wasm32 `simd128`
// backend only activates when the module is built with the `+simd128` target
// feature. This benchmark is how the SIMD build's win is measured: it prints a
// compact, greppable summary line you compare against a prior run to spot a
// throughput regression. It is report-only — it asserts correctness (parsed
// size) but never gates on throughput and stores no baseline, so a regression
// is caught by reading the numbers, not automatically. wasm SIMD codegen only
// exists once the module runs in V8, so a native criterion bench cannot see it.
//
// Usage:
//   node bench/parse.bench.mjs
// Tunables (env):
//   BENCH_TRIPLES  number of triples in the corpus   (default 150000)
//   BENCH_ITERS    timed iterations after warm-up     (default 7)
//   BENCH_WARMUP   warm-up iterations (discarded)     (default 2)

import { ready, CompiledJsonLdContext, Dataset } from "../index.mjs";

await ready();

const TRIPLES = Number(process.env.BENCH_TRIPLES ?? 150_000);
const ITERS = Number(process.env.BENCH_ITERS ?? 7);
const WARMUP = Number(process.env.BENCH_WARMUP ?? 2);

// Deterministic corpus generation — no Math.random, no fixture files (nothing
// under vectors/ is touched). The shape mixes IRI objects, plain string
// literals, language-tagged literals, and integer-typed literals so the parser
// exercises delimiter/newline scanning across varied line lengths.
function buildCorpus(n) {
  const S = "https://example.org/s/";
  const P = "https://example.org/p/";
  const O = "https://example.org/o/";
  const XSD_INT = "http://www.w3.org/2001/XMLSchema#integer";
  const parts = [];
  for (let i = 0; i < n; i++) {
    const s = `<${S}${i}>`;
    const p = `<${P}${i % 32}>`;
    switch (i % 4) {
      case 0:
        parts.push(`${s} ${p} <${O}${i}> .\n`);
        break;
      case 1:
        parts.push(`${s} ${p} "literal value number ${i} with several words" .\n`);
        break;
      case 2:
        parts.push(`${s} ${p} "language tagged value ${i}"@en .\n`);
        break;
      default:
        parts.push(`${s} ${p} "${i}"^^<${XSD_INT}> .\n`);
        break;
    }
  }
  return parts.join("");
}

function median(xs) {
  const s = [...xs].sort((a, b) => a - b);
  const mid = s.length >> 1;
  return s.length % 2 ? s[mid] : (s[mid - 1] + s[mid]) / 2;
}

const corpus = buildCorpus(TRIPLES);
const bytes = Buffer.byteLength(corpus, "utf8");

// Warm up (JIT + wasm instance warm caches); results discarded.
for (let i = 0; i < WARMUP; i++) {
  const ds = Dataset.parse(corpus, "ntriples");
  if (ds.size !== TRIPLES) {
    throw new Error(`corpus size mismatch: parsed ${ds.size}, expected ${TRIPLES}`);
  }
}

const samples = [];
for (let i = 0; i < ITERS; i++) {
  const t0 = process.hrtime.bigint();
  const ds = Dataset.parse(corpus, "ntriples");
  const t1 = process.hrtime.bigint();
  // Touch size so the parse cannot be optimized away.
  if (ds.size !== TRIPLES) {
    throw new Error(`corpus size mismatch: parsed ${ds.size}, expected ${TRIPLES}`);
  }
  samples.push(Number(t1 - t0) / 1e6); // ms
}

const med = median(samples);
const min = Math.min(...samples);
const mb = bytes / (1024 * 1024);
const triplesPerSec = TRIPLES / (med / 1000);
const mbPerSec = mb / (med / 1000);

const fmt = (x, d = 2) => x.toFixed(d);
// Compact, greppable summary line (grep 'BENCH parse').
console.log(
  `BENCH parse ntriples: triples=${TRIPLES} bytes=${bytes} ` +
    `iters=${ITERS} median_ms=${fmt(med)} min_ms=${fmt(min)} ` +
    `triples_per_s=${fmt(triplesPerSec, 0)} MB_per_s=${fmt(mbPerSec)}`,
);

const contextOptions = JSON.stringify({
  version: 1,
  mode: "context",
  prefixes: {
    s: "https://example.org/s/",
    p: "https://example.org/p/",
    o: "https://example.org/o/",
  },
});
const compileSamples = [];
for (let i = 0; i < ITERS; i++) {
  const t0 = process.hrtime.bigint();
  const context = new CompiledJsonLdContext(contextOptions);
  const t1 = process.hrtime.bigint();
  compileSamples.push(Number(t1 - t0) / 1e6);
  context.free();
}
const dataset = Dataset.parse(corpus, "ntriples");
const context = new CompiledJsonLdContext(contextOptions);
const serializeSamples = [];
let compacted = "";
for (let i = 0; i < ITERS; i++) {
  const t0 = process.hrtime.bigint();
  compacted = dataset.serializeWithContext("jsonld", context);
  const t1 = process.hrtime.bigint();
  serializeSamples.push(Number(t1 - t0) / 1e6);
}
const parseSamples = [];
for (let i = 0; i < ITERS; i++) {
  const t0 = process.hrtime.bigint();
  const reparsed = Dataset.parse(compacted, "jsonld");
  const t1 = process.hrtime.bigint();
  if (reparsed.size !== TRIPLES) throw new Error("configured JSON-LD size mismatch");
  parseSamples.push(Number(t1 - t0) / 1e6);
  reparsed.free();
}
console.log(
  `BENCH jsonld configured: triples=${TRIPLES} output_bytes=${Buffer.byteLength(compacted)} ` +
    `compile_median_ms=${fmt(median(compileSamples), 4)} ` +
    `serialize_median_ms=${fmt(median(serializeSamples))} ` +
    `parse_median_ms=${fmt(median(parseSamples))}`,
);
