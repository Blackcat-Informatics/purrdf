// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

/**
 * Compile emitted declarations with the locked TypeScript 7.0 compiler and
 * compare their JSON-literal acceptance with dev-only boon classifications.
 */

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const REPO = fileURLToPath(new URL("../../..", import.meta.url));
const TYPESCRIPT_PROJECT = path.join(REPO, "crates", "rdf-wasm", "js");
const TYPESCRIPT_VERSION = "7.0.2";
const CLOSED_PROFILE = new Set([
  "additional-properties-validation-widened",
  "array-cardinality-validation-widened",
  "array-contains-validation-dropped",
  "conditional-validation-dropped",
  "dependency-validation-dropped",
  "integer-validation-widened",
  "keyword-validation-dropped",
  "negation-validation-dropped",
  "numeric-validation-dropped",
  "object-literal-validation-widened",
  "one-of-validation-widened",
  "pattern-properties-validation-dropped",
  "property-count-validation-dropped",
  "property-name-validation-dropped",
  "string-validation-dropped",
  "tuple-array-validation-widened",
  "unevaluated-validation-dropped",
  "unique-items-validation-dropped",
]);

const requireFromToolchain = createRequire(
  path.join(TYPESCRIPT_PROJECT, "package.json"),
);
const compilerApiUrl = pathToFileURL(
  requireFromToolchain.resolve("typescript/unstable/sync"),
).href;
const versionUrl = pathToFileURL(
  requireFromToolchain.resolve("typescript"),
).href;
const { API } = await import(compilerApiUrl);
const versionModule = await import(versionUrl);
const compilerVersion =
  versionModule.version ?? versionModule.default?.version ?? versionModule.default;

function fixtureManifest() {
  const completed = spawnSync(
    "cargo",
    [
      "run",
      "-p",
      "purrdf-shapes",
      "--example",
      "typescript_oracle_fixture",
      "--locked",
      "--quiet",
    ],
    {
      cwd: REPO,
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    },
  );
  if (completed.status !== 0) {
    throw new Error(
      `TypeScript oracle fixture failed (${completed.status}):\n${completed.stderr}`,
    );
  }
  return JSON.parse(completed.stdout);
}

function sourceExpression(probe) {
  const literal = JSON.stringify(probe.value);
  if (
    probe.mode === "variable" &&
    probe.value !== null &&
    typeof probe.value === "object"
  ) {
    return [
      `const candidate = ${literal} as const;`,
      "const value: Target = candidate;",
      "void value;",
    ].join("\n");
  }
  if (probe.mode === "variable") {
    return [
      `const candidate = ${literal};`,
      "const value: Target = candidate;",
      "void value;",
    ].join("\n");
  }
  if (probe.mode !== "fresh") {
    throw new Error(`unknown probe mode ${JSON.stringify(probe.mode)}`);
  }
  return [`const value: Target = ${literal};`, "void value;"].join("\n");
}

function probeSource(probe, compilerOnly = false) {
  const assignment = compilerOnly
    ? [`const value: Target = (${probe.expression});`, "void value;"].join("\n")
    : sourceExpression(probe);
  return [
    `import type { ${probe.typeName} as Target } from "./index.js";`,
    assignment,
    "export {};",
    "",
  ].join("\n");
}

function diagnosticsText(diagnostics) {
  return diagnostics
    .map((diagnostic) => {
      const file = diagnostic.fileName ? path.basename(diagnostic.fileName) : "<project>";
      return `${file}: TS${diagnostic.code}: ${diagnostic.text}`;
    })
    .join("\n");
}

function compileFixture(name, fixture, directory) {
  const root = path.join(directory, name);
  const files = ["index.d.ts"];
  mkdirSync(root, { recursive: true });
  writeFileSync(path.join(root, "index.d.ts"), fixture.declaration, "utf8");
  writeFileSync(path.join(root, "package.json"), '{"type":"module"}\n', "utf8");

  const probes = [];
  fixture.probes.forEach((probe, index) => {
    const file = `probe-${String(index).padStart(3, "0")}.ts`;
    files.push(file);
    probes.push({ file, probe, compilerOnly: false });
    writeFileSync(path.join(root, file), probeSource(probe), "utf8");
  });
  fixture.compilerProbes.forEach((probe, index) => {
    const file = `compiler-${String(index).padStart(3, "0")}.ts`;
    files.push(file);
    probes.push({ file, probe, compilerOnly: true });
    writeFileSync(path.join(root, file), probeSource(probe, true), "utf8");
  });

  const configPath = path.join(root, "tsconfig.json");
  writeFileSync(
    configPath,
    `${JSON.stringify(
      {
        compilerOptions: {
          target: "ES2022",
          module: "NodeNext",
          moduleResolution: "NodeNext",
          strict: true,
          exactOptionalPropertyTypes: true,
          noUncheckedIndexedAccess: true,
          noEmit: true,
          skipLibCheck: false,
          forceConsistentCasingInFileNames: true,
          types: [],
        },
        files,
      },
      null,
      2,
    )}\n`,
    "utf8",
  );

  const api = new API({ cwd: root });
  let snapshot;
  try {
    snapshot = api.updateSnapshot({ openProjects: [configPath] });
    const projects = snapshot.getProjects();
    const project = snapshot.getProject(configPath) ?? projects[0];
    if (project === undefined || projects.length !== 1) {
      throw new Error(
        `${name} compiler oracle expected one project, found ${projects.length}`,
      );
    }
    const program = project.program;
    const baselineDiagnostics = [
      ...program.getConfigFileParsingDiagnostics(),
      ...program.getProgramDiagnostics(),
      ...program.getGlobalDiagnostics(),
      ...program.getSyntacticDiagnostics(path.join(root, "index.d.ts")),
      ...program.getBindDiagnostics(path.join(root, "index.d.ts")),
      ...program.getSemanticDiagnostics(path.join(root, "index.d.ts")),
    ];
    if (baselineDiagnostics.length !== 0) {
      throw new Error(
        `${name} declaration or compiler configuration is invalid:\n${diagnosticsText(
          baselineDiagnostics,
        )}`,
      );
    }

    return probes.map(({ file, probe, compilerOnly }) => {
      const filePath = path.join(root, file);
      const diagnostics = [
        ...program.getSyntacticDiagnostics(filePath),
        ...program.getBindDiagnostics(filePath),
        ...program.getSemanticDiagnostics(filePath),
      ];
      return {
        probe,
        compilerOnly,
        valid: diagnostics.length === 0,
        diagnostics,
      };
    });
  } finally {
    snapshot?.dispose();
    api.close();
  }
}

function lossSubject(entry) {
  const marker = " subject=";
  const start = entry.location?.indexOf(marker);
  if (start === undefined || start < 0) {
    throw new Error(`loss ${entry.code} has no logical subject: ${entry.location}`);
  }
  return entry.location.slice(start + marker.length);
}

function lossKey(loss) {
  return `${loss.code}\u0000${loss.location}`;
}

function compareProbe(fixtureName, probe, actual, locatedLosses) {
  if (probe.expectedLoss === undefined) {
    if (actual !== probe.sourceValid) {
      throw new Error(
        `${fixtureName}/${probe.label} has an unlocated acceptance divergence: ` +
          `boon=${probe.sourceValid}, TypeScript=${actual}`,
      );
    }
    return;
  }
  if (!locatedLosses.has(lossKey(probe.expectedLoss))) {
    throw new Error(
      `${fixtureName}/${probe.label} names a missing ledger entry ` +
        `${probe.expectedLoss.code} at ${probe.expectedLoss.location}`,
    );
  }
  if (actual === probe.sourceValid) {
    throw new Error(
      `${fixtureName}/${probe.label} was expected to expose ` +
        `${probe.expectedLoss.code}, but boon and TypeScript both classified it as ${actual}`,
    );
  }
}

function assertSelfTest() {
  assert.throws(
    () =>
      compareProbe(
        "self-test",
        { label: "flipped", sourceValid: true },
        false,
        new Set(),
      ),
    /unlocated acceptance divergence/,
  );
  assert.throws(
    () =>
      compareProbe(
        "self-test",
        {
          label: "missing-loss",
          sourceValid: false,
          expectedLoss: { code: "absent", location: "#/absent" },
        },
        true,
        new Set(),
      ),
    /missing ledger entry/,
  );
  assert.throws(
    () =>
      compareProbe(
        "self-test",
        {
          label: "hidden-divergence",
          sourceValid: false,
          expectedLoss: { code: "present", location: "#/present" },
        },
        false,
        new Set(["present\u0000#/present"]),
      ),
    /expected to expose/,
  );
}

function assertFixture(name, fixture, results) {
  const losses = fixture.losses.losses;
  if (!Array.isArray(losses) || !losses.every((entry) => entry.intentional === true)) {
    throw new Error(`${name} fixture has a malformed or unregistered loss ledger`);
  }
  const locatedLosses = new Set(
    losses.map((entry) => lossKey({ code: entry.code, location: lossSubject(entry) })),
  );
  for (const result of results) {
    if (result.compilerOnly) {
      if (result.valid !== result.probe.expectedTypescriptValid) {
        throw new Error(
          `${name}/${result.probe.label} compiler-only expectation differs: ` +
            `expected=${result.probe.expectedTypescriptValid}, actual=${result.valid}\n` +
            diagnosticsText(result.diagnostics),
        );
      }
    } else {
      try {
        compareProbe(name, result.probe, result.valid, locatedLosses);
      } catch (error) {
        error.message += `\n${diagnosticsText(result.diagnostics)}`;
        throw error;
      }
    }
  }
}

if (compilerVersion !== TYPESCRIPT_VERSION) {
  throw new Error(
    `TypeScript compiler version drift: expected ${TYPESCRIPT_VERSION}, found ${compilerVersion}`,
  );
}
assertSelfTest();
if (process.argv.includes("--self-test")) {
  console.log(
    "TypeScript oracle self-test: flipped, unlocated, and hidden divergences were rejected",
  );
  process.exit(0);
}
const manifest = fixtureManifest();
if (!manifest.reverse.shapeIds.includes("<https://example.org/Person>")) {
  throw new Error(`TypeScript reverse import lost Person: ${manifest.reverse.shapeIds}`);
}
if (
  !manifest.reverse.losses.losses.every(
    (entry) =>
      entry.from === "typescript-7.0" &&
      entry.to === "shacl" &&
      entry.intentional === true &&
      lossSubject(entry).startsWith("#/"),
  )
) {
  throw new Error("TypeScript reverse package has an unsound or unlocated loss");
}
if (manifest.exact.losses.losses.length !== 0) {
  throw new Error("exact TypeScript oracle fixture has a non-empty loss ledger");
}
const lossyCodes = new Set(manifest.lossy.losses.losses.map((entry) => entry.code));
assert.deepEqual(lossyCodes, CLOSED_PROFILE, "lossy fixture no longer covers the closed profile");

const directory = mkdtempSync(path.join(tmpdir(), "purrdf-typescript-oracle-"));
try {
  const exactResults = compileFixture("exact", manifest.exact, directory);
  const lossyResults = compileFixture("lossy", manifest.lossy, directory);
  assertFixture("exact", manifest.exact, exactResults);
  assertFixture("lossy", manifest.lossy, lossyResults);
  const divergenceCount = lossyResults.filter(
    ({ probe, compilerOnly, valid }) => !compilerOnly && valid !== probe.sourceValid,
  ).length;
  if (divergenceCount < 16) {
    throw new Error(`lossy fixture exposed only ${divergenceCount} located divergences`);
  }
  console.log(
    `TypeScript oracle: compiler ${compilerVersion}; ` +
      `${manifest.exact.probes.length} exact boon probes and ` +
      `${manifest.exact.compilerProbes.length} optional/null/undefined probes agree; ` +
      `${divergenceCount} divergences map to the complete 18-code loss profile; ` +
      "verified reverse SHACL import passes",
  );
} finally {
  rmSync(directory, { recursive: true, force: true });
}
