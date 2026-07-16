// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

/**
 * Validate generated SDL and execute real GraphQL variable coercion through
 * the locked official GraphQL.js implementation, with boon as source truth.
 */

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REPO = fileURLToPath(new URL("../../..", import.meta.url));
const TOOLCHAIN = path.join(REPO, "crates", "rdf-wasm", "js");
const GRAPHQL_VERSION = "16.14.0";
const requireFromToolchain = createRequire(path.join(TOOLCHAIN, "package.json"));
const {
  buildSchema,
  graphql,
  validateSchema,
  valueFromASTUntyped,
  version: graphqlVersion,
} = requireFromToolchain("graphql");

function fixtureManifest() {
  const completed = spawnSync(
    "cargo",
    [
      "run",
      "-p",
      "purrdf-shapes",
      "--example",
      "graphql_oracle_fixture",
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
      `GraphQL oracle fixture failed (${completed.status}):\n${completed.stderr}`,
    );
  }
  return JSON.parse(completed.stdout);
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
          `boon=${probe.sourceValid}, GraphQL=${actual}`,
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
        `${probe.expectedLoss.code}, but boon and GraphQL both classified it as ${actual}`,
    );
  }
}

function assertNameMap(fixture) {
  const artifact = JSON.parse(fixture.nameMapArtifact);
  assert.deepEqual(
    artifact,
    fixture.nameMap,
    "typed GraphQL name map differs from name-map.json",
  );
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
          label: "missing-location",
          sourceValid: false,
          expectedLoss: { code: "present", location: "#/missing" },
        },
        true,
        new Set(["present\u0000#/present"]),
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
  assert.throws(
    () =>
      assertNameMap({
        nameMapArtifact: '{"schema_name":"expected"}\n',
        nameMap: { schema_name: "corrupted" },
      }),
    /typed GraphQL name map differs/,
  );
}

function buildFixtureSchema(fixture) {
  const queryFields = fixture.probes
    .map((probe, index) => `  probe${index}(value: ${probe.graphqlType}): Boolean!`)
    .join("\n");
  const schema = buildSchema(`${fixture.sdl}\ntype Query {\n${queryFields}\n}\n`);
  const scalar = schema.getType(fixture.fallbackScalar);
  if (scalar === undefined || scalar.constructor.name !== "GraphQLScalarType") {
    throw new Error(`fixture fallback scalar ${fixture.fallbackScalar} is absent`);
  }
  scalar.serialize = (value) => value;
  scalar.parseValue = (value) => value;
  scalar.parseLiteral = (node, variables) => valueFromASTUntyped(node, variables);
  const errors = validateSchema(schema);
  if (errors.length !== 0) {
    throw new Error(
      `generated GraphQL schema is invalid:\n${errors.map((error) => error.message).join("\n")}`,
    );
  }
  return schema;
}

async function executeFixture(name, fixture) {
  assertNameMap(fixture);
  const losses = fixture.losses.losses;
  if (!Array.isArray(losses) || !losses.every((entry) => entry.intentional === true)) {
    throw new Error(`${name} fixture has a malformed or unregistered loss ledger`);
  }
  const locatedLosses = new Set(
    losses.map((entry) => lossKey({ code: entry.code, location: lossSubject(entry) })),
  );
  const schema = buildFixtureSchema(fixture);
  const results = [];
  for (const [index, probe] of fixture.probes.entries()) {
    const field = `probe${index}`;
    const result = await graphql({
      schema,
      source: `query Probe($value: ${probe.graphqlType}) { ${field}(value: $value) }`,
      rootValue: { [field]: () => true },
      variableValues: { value: probe.graphqlValue },
    });
    const valid = result.errors === undefined;
    try {
      compareProbe(name, probe, valid, locatedLosses);
    } catch (error) {
      const diagnostics = result.errors?.map((entry) => entry.message).join("\n") ?? "";
      error.message += diagnostics.length === 0 ? "" : `\n${diagnostics}`;
      throw error;
    }
    results.push({ probe, valid });
  }
  return results;
}

if (graphqlVersion !== GRAPHQL_VERSION) {
  throw new Error(
    `GraphQL.js version drift: expected ${GRAPHQL_VERSION}, found ${graphqlVersion}`,
  );
}
assertSelfTest();
if (process.argv.includes("--self-test")) {
  console.log(
    "GraphQL oracle self-test: flipped, unlocated, hidden, and name-map corruptions were rejected",
  );
  process.exit(0);
}

const manifest = fixtureManifest();
if (manifest.dialect !== "graphql-september-2025") {
  throw new Error(`GraphQL dialect drift: ${manifest.dialect}`);
}
if (manifest.exact.losses.losses.length !== 0) {
  throw new Error("exact GraphQL oracle fixture has a non-empty loss ledger");
}
const profile = new Set(manifest.closedProfile);
const lossyCodes = new Set(manifest.lossy.losses.losses.map((entry) => entry.code));
assert.deepEqual(lossyCodes, profile, "lossy fixture no longer covers the closed profile");

const exactResults = await executeFixture("exact", manifest.exact);
const lossyResults = await executeFixture("lossy", manifest.lossy);
const divergenceCount = lossyResults.filter(
  ({ probe, valid }) => valid !== probe.sourceValid,
).length;
if (divergenceCount !== manifest.lossy.probes.length) {
  throw new Error(
    `lossy fixture exposed ${divergenceCount}/${manifest.lossy.probes.length} divergences`,
  );
}
const codecCount = [...manifest.exact.probes, ...manifest.lossy.probes].filter(
  (probe) => probe.usedCodec,
).length;
console.log(
  `GraphQL oracle: GraphQL.js ${graphqlVersion}; ` +
    `${exactResults.length} exact boon/variable-coercion probes agree; ` +
    `${divergenceCount} located divergences cover the complete ${profile.size}-code profile; ` +
    `${codecCount} probes exercise the production value codec`,
);
