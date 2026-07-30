// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Packed-package smoke gate for the npm artifact. This validates the exact tarball
// npm would publish: pack, install into a clean project, import by package name, and
// exercise the package-root API over the optimized wasm artifact.

import { execFileSync } from "node:child_process";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

import { parsePackument } from "./npm-pack-output.mjs";

const PACKAGE_ROOT = resolve(fileURLToPath(new URL("..", import.meta.url)));
// The wasm package ships the RDF 1.2 model, SPARQL/SHACL/ShEx engines, the
// native format registry (Turtle/N-Quads/TriG/JSON-LD/YAML-LD/…), layout, the
// SVG renderer, and all sixteen graph/tabular/dataset-description/research-object
// projection profiles. Both ceilings track the optimized wasm artifact (see the
// Makefile WASM_SIZE_BUDGET_BYTES note); each is the measured size plus about 3%
// headroom. The five strict bidirectional research-object codecs, configured
// JSON-LD context engine, and scoped LPG mapper account for earlier reviewed
// increases. The always-on curated CSVW and OKF terms mappers, their closed
// located-loss contracts, and shared host dispatch account for one increase;
// bounded CONSTRUCT, mapped native DCAT RDF, and VoID generation account for
// one increase; validation-scoped asserted-subclass membership shared by native
// SHACL and SHACL-SPARQL accounts for one. The latest is the reasoning surface:
// the exported entailment API in crates/rdf-wasm/src/entail.rs links
// purrdf-entail and purrdf-datalog into the wasm artifact, so both ceilings move
// with the WASM_SIZE_BUDGET_BYTES raise that records it. The most recent is the
// concrete domain, which moved the wasm artifact and both of these with it.
//
// The MEASURED figures below are gated by EQUALITY, not treated as prose. The
// pair that used to sit in this comment fell 226_581 and 670_516 bytes behind the
// build while the sentence still claimed "about 3% headroom" — the real figures
// were 0.170% and 0.056%, which is 5_441 and 5_374 bytes from a red gate. A
// ceiling only speaks when it is crossed, so it cannot report drift underneath
// itself; an equality does, and it forces the commit that moved the package to
// say so. Raise the ceilings deliberately, the same way the Makefile's are
// raised: rebuild, read the printed size, restore a few percent of headroom, and
// state in the commit which capability grew the package.
const MEASURED_TARBALL_BYTES = 3_206_587;
const MEASURED_UNPACKED_BYTES = 9_614_663;
const MAX_TARBALL_BYTES = 3_310_000;
const MAX_UNPACKED_BYTES = 9_920_000;
const DEFAULT_COMMAND_TIMEOUT_MS = 120_000;
const NPM_INSTALL_TIMEOUT_MS = 180_000;
const SMOKE_TIMEOUT_MS = 60_000;

/**
 * Fail unless `actual` equals the recorded measurement exactly.
 *
 * Not a ceiling: any change to the packaged bytes fails here until the recorded figure moves
 * in the same commit. That is the point — it converts "someone will notice the package grew"
 * into a red gate, and it is why the attribution in the comment above can be trusted.
 */
function assertMeasured(label, actual, recorded) {
  if (actual !== recorded) {
    throw new Error(
      `${label} measures ${actual} bytes but this file records ${recorded}. ` +
        `The recorded size is not a ceiling — it is the measurement this package publishes. ` +
        `Set it to ${actual} in the same commit that moved the package, and say WHY it moved. ` +
        `If the move also crosses the ceiling, restore a few percent of headroom deliberately ` +
        `rather than raising it to go green.`,
    );
  }
}

function run(command, args, options = {}) {
  const { timeout = DEFAULT_COMMAND_TIMEOUT_MS, ...execOptions } = options;
  return execFileSync(command, args, {
    cwd: PACKAGE_ROOT,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
    shell: process.platform === "win32" && command === "npm",
    timeout,
    ...execOptions,
  });
}

function assertBudget(name, size, budget) {
  if (size > budget) {
    throw new Error(`${name} ${size} bytes exceeds budget ${budget} bytes`);
  }
}

async function writeSummary(packument) {
  const lines = [
    `npm tarball: ${packument.size} bytes / budget ${MAX_TARBALL_BYTES} bytes`,
    `npm unpacked: ${packument.unpackedSize} bytes / budget ${MAX_UNPACKED_BYTES} bytes`,
    `npm entries: ${packument.entryCount}`,
  ];
  console.log(lines.join("\n"));
  if (process.env.GITHUB_STEP_SUMMARY) {
    await writeFile(
      process.env.GITHUB_STEP_SUMMARY,
      `### npm package size\n\n${lines.map((line) => `- ${line}`).join("\n")}\n`,
      { flag: "a" },
    );
  }
}

const smokeProgram = String.raw`
import assert from "node:assert/strict";
import {
  ready,
  DataFactory,
  Dataset,
  QueryEngine,
  shaclValidateToSarif,
} from "@blackcatinformatics/purrdf";

await ready();

const f = new DataFactory();
const subject = f.namedNode("https://example.org/stmt");
const predicate = f.namedNode("https://example.org/says");
const quoted = f.quotedTriple(
  f.namedNode("https://example.org/alice"),
  f.namedNode("https://example.org/knows"),
  f.namedNode("https://example.org/bob"),
);
const directional = f.literal("مرحبا", { language: "ar", direction: "rtl" });
const dataset = Dataset.from([f.quad(subject, predicate, directional)]);
dataset.add(f.quad(subject, f.namedNode("https://example.org/source"), quoted));

const nquads = dataset.serialize("nquads");
const reparsed = Dataset.parse(nquads, "nquads");
assert.equal(reparsed.size, 2);
assert.equal(reparsed.quads().some((quad) => quad.object.direction === "rtl"), true);
assert.equal(dataset.isomorphic(reparsed), true);
assert.equal(dataset.canonicalize(), reparsed.canonicalize());

const engine = new QueryEngine();
const select = engine.select(
  reparsed,
  "PREFIX ex: <https://example.org/> SELECT ?msg WHERE { ex:stmt ex:says ?msg }",
);
assert.equal(select.kind, "select");
assert.equal(select.rows.take(0).msg.direction, "rtl");
assert.equal(
  engine.ask(reparsed, "PREFIX ex: <https://example.org/> ASK { ex:stmt ex:says ?msg }"),
  true,
);
const graph = engine.construct(
  reparsed,
  "PREFIX ex: <https://example.org/> CONSTRUCT { ex:copy ex:says ?msg } WHERE { ex:stmt ex:says ?msg }",
);
assert.equal(graph.size, 1);
assert.match(
  engine.queryRaw(reparsed, "PREFIX ex: <https://example.org/> ASK { ex:stmt ex:says ?msg }", {
    format: "xml",
  }),
  /^<\?xml/,
);

const mutable = new Dataset();
engine.update(
  mutable,
  "INSERT DATA { <https://example.org/u> <https://example.org/p> <https://example.org/o> }",
);
assert.equal(mutable.size, 1);
const beforeFailedUpdate = mutable.canonicalize();
assert.throws(() =>
  engine.update(
    mutable,
    "INSERT DATA { <https://example.org/x> <https://example.org/p> <https://example.org/y> } ; LOAD <https://example.org/doc>",
  ),
);
assert.equal(mutable.canonicalize(), beforeFailedUpdate);

const shapes = [
  "@prefix sh: <http://www.w3.org/ns/shacl#> .",
  "@prefix ex: <http://example.org/> .",
  "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .",
  "ex:PersonShape a sh:NodeShape ;",
  "  sh:targetClass ex:Person ;",
  "  sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .",
  "",
].join("\n");
const data = [
  '<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .',
  '<http://example.org/alice> <http://example.org/age> "nope" .',
  "",
].join("\n");
const sarif = JSON.parse(shaclValidateToSarif(shapes, data));
assert.equal(sarif.version, "2.1.0");
assert.ok(sarif.runs.flatMap((run) => run.results ?? []).length >= 1);
`;

const root = await mkdtemp(join(tmpdir(), "purrdf-pack-smoke-"));
try {
  const packOutput = run("npm", ["pack", "--json", "--pack-destination", root]);
  const packument = parsePackument(packOutput);
  assertBudget("tarball", packument.size, MAX_TARBALL_BYTES);
  assertBudget("unpacked package", packument.unpackedSize, MAX_UNPACKED_BYTES);
  assertMeasured("tarball", packument.size, MEASURED_TARBALL_BYTES);
  assertMeasured("unpacked package", packument.unpackedSize, MEASURED_UNPACKED_BYTES);
  await writeSummary(packument);

  const project = join(root, "project");
  await mkdir(project);
  await writeFile(
    join(project, "package.json"),
    JSON.stringify({ private: true, type: "module" }, null, 2),
  );
  const tarball = join(root, packument.filename);
  run("npm", ["install", "--ignore-scripts", "--no-audit", "--no-fund", tarball], {
    cwd: project,
    stdio: "inherit",
    timeout: NPM_INSTALL_TIMEOUT_MS,
  });

  const smokePath = join(project, "smoke.mjs");
  await writeFile(smokePath, smokeProgram);
  run(process.execPath, [smokePath], {
    cwd: project,
    stdio: "inherit",
    timeout: SMOKE_TIMEOUT_MS,
  });
  console.log(`OK: packed tarball smoke passed for ${packument.filename}`);
} finally {
  await rm(root, { force: true, recursive: true });
}
