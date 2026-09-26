// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
  getNamedType,
  graphql,
  isObjectType,
  isUnionType,
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

/** Flatten nested lists to the non-null values they hold. */
function leaves(values) {
  return values.flatMap((value) =>
    Array.isArray(value) ? leaves(value) : value === null || value === undefined ? [] : [value],
  );
}

/**
 * The selection set that reads back exactly the fields `values` carry: an
 * object type selects each present field, a union selects `__typename` and
 * one inline fragment per member the values name.
 */
function selection(type, values) {
  const named = getNamedType(type);
  const objects = leaves(values).filter((value) => typeof value === "object");
  if (isUnionType(named)) {
    const members = new Map();
    for (const value of objects) {
      const member = value.__typename;
      if (typeof member !== "string") {
        throw new Error(`union ${named.name} output value lacks __typename`);
      }
      members.set(member, [...(members.get(member) ?? []), value]);
    }
    const fragments = [...members.entries()]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([member, group]) => `... on ${member} ${selection(schemaType(named, member), group)}`);
    return `{ __typename ${fragments.join(" ")} }`;
  }
  if (!isObjectType(named)) {
    return "";
  }
  const fieldNames = [...new Set(objects.flatMap((value) => Object.keys(value)))]
    .filter((name) => name !== "__typename")
    .sort();
  const fields = named.getFields();
  const selected = fieldNames.map((name) => {
    if (fields[name] === undefined) {
      throw new Error(`output value field ${name} is not declared by ${named.name}`);
    }
    return `${name} ${selection(fields[name].type, objects.map((value) => value[name]))}`;
  });
  const typename = objects.some((value) => "__typename" in value) ? "__typename " : "";
  return `{ ${typename}${selected.join(" ")} }`;
}

let activeSchema;
function schemaType(union, member) {
  const type = activeSchema.getType(member);
  if (!union.getTypes().includes(type)) {
    throw new Error(`${member} is not a member of union ${union.name}`);
  }
  return type;
}

function buildFixtureSchema(fixture) {
  const queryFields = fixture.probes
    .flatMap((probe, index) => [
      `  probe${index}(value: ${probe.graphqlType}): Boolean!`,
      ...(probe.output === undefined ? [] : [`  output${index}: ${probe.output.graphqlType}`]),
    ])
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
    if (probe.output !== undefined) {
      await assertOutput(name, schema, index, probe);
    }
  }
  return results;
}

/**
 * A resolver returning the codec's output value for a valid source value is
 * serialized by GraphQL.js unchanged, union members resolved by __typename.
 */
async function assertOutput(name, schema, index, probe) {
  activeSchema = schema;
  const field = `output${index}`;
  const type = schema.getQueryType().getFields()[field].type;
  const result = await graphql({
    schema,
    source: `query Output { ${field} ${selection(type, [probe.output.graphqlValue])} }`,
    rootValue: { [field]: () => probe.output.graphqlValue },
  });
  if (result.errors !== undefined) {
    throw new Error(
      `${name}/${probe.label} output was not serialized:\n` +
        result.errors.map((entry) => entry.message).join("\n"),
    );
  }
  assert.deepEqual(
    JSON.parse(JSON.stringify(result.data[field])),
    probe.output.graphqlValue,
    `${name}/${probe.label} output changed through GraphQL.js serialization`,
  );
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
if (!manifest.reverse.shapeIds.includes("<https://example.org/Person>")) {
  throw new Error(`GraphQL reverse import lost Person: ${manifest.reverse.shapeIds}`);
}
if (
  !manifest.reverse.losses.losses.every(
    (entry) =>
      entry.from === "graphql-september-2025" &&
      entry.to === "shacl" &&
      entry.intentional === true &&
      lossSubject(entry).startsWith("#/"),
  )
) {
  throw new Error("GraphQL reverse package has an unsound or unlocated loss");
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
// The SHACL list components over projected instances: a list value is a
// @oneOf input of a node reference and a @list object whose members are a
// @oneOf input of their alternatives, so member typing and list-ness agree
// with SHACL validation; a probe diverges only at a located length,
// uniqueness or numeric-bound loss GraphQL has no expression for.
const listResults = await executeFixture("lists", manifest.lists);
const listDivergences = listResults.filter(({ probe, valid }) => valid !== probe.sourceValid);
assert.deepEqual(
  listDivergences.map(({ probe }) => probe.label),
  manifest.lists.probes.filter((probe) => probe.expectedLoss !== undefined).map((probe) => probe.label),
  "lists fixture divergences drifted",
);
for (const label of ["member-not-integer", "bounded-not-a-list"]) {
  const agreed = listResults.find(({ probe }) => probe.label === label);
  assert.ok(agreed !== undefined && !agreed.valid, `${label} must be rejected by GraphQL.js`);
}
const outputCount = [manifest.exact, manifest.lossy, manifest.lists]
  .flatMap((fixture) => fixture.probes)
  .filter((probe) => probe.output !== undefined).length;
const codecCount = [...manifest.exact.probes, ...manifest.lossy.probes].filter(
  (probe) => probe.usedCodec,
).length;
console.log(
  `GraphQL oracle: GraphQL.js ${graphqlVersion}; ` +
    `${exactResults.length} exact boon/variable-coercion probes agree; ` +
    `${divergenceCount} located divergences cover the complete ${profile.size}-code profile; ` +
    `${codecCount} probes exercise the production value codec; ` +
    `${listResults.length} SHACL list-component probes agree or diverge at a located loss ` +
    `(${listDivergences.length} divergences); ` +
    `${outputCount} valid values serialize unchanged through output types and unions; ` +
    "verified reverse SHACL import passes",
);
