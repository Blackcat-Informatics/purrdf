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

const PACKAGE_ROOT = resolve(fileURLToPath(new URL("..", import.meta.url)));
const MAX_TARBALL_BYTES = 1_400_000;
const MAX_UNPACKED_BYTES = 3_600_000;

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    cwd: PACKAGE_ROOT,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
    ...options,
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
assert.equal(select.rows[0].msg.direction, "rtl");
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
  const [packument] = JSON.parse(packOutput);
  if (!packument) throw new Error("npm pack did not return a package record");
  assertBudget("tarball", packument.size, MAX_TARBALL_BYTES);
  assertBudget("unpacked package", packument.unpackedSize, MAX_UNPACKED_BYTES);
  await writeSummary(packument);

  const project = join(root, "project");
  await mkdir(project);
  await writeFile(
    join(project, "package.json"),
    JSON.stringify({ private: true, type: "module" }, null, 2),
  );
  const tarball = join(root, packument.filename);
  execFileSync("npm", ["install", "--ignore-scripts", "--no-audit", "--no-fund", tarball], {
    cwd: project,
    stdio: "inherit",
  });

  const smokePath = join(project, "smoke.mjs");
  await writeFile(smokePath, smokeProgram);
  execFileSync(process.execPath, [smokePath], { cwd: project, stdio: "inherit" });
  console.log(`OK: packed tarball smoke passed for ${packument.filename}`);
} finally {
  await rm(root, { force: true, recursive: true });
}
