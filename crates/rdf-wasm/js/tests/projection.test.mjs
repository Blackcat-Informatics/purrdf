// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import assert from "node:assert/strict";

import { Dataset, liftProjection, ready } from "../index.mjs";

await ready();

const CONFIG = JSON.stringify({
  profile: "lpg-csv",
  config: {
    rdf_type: "https://example.org/type",
    scope: { mode: "all" },
    limits: {
      max_artifacts: 16,
      max_artifact_bytes: 1_000_000,
      max_total_bytes: 4_000_000,
      max_archive_bytes: 5_000_000,
      max_term_depth: 16,
    },
    execution_limits: {
      max_input_records: 1_000,
      max_model_records: 1_000,
      max_nodes: 1_000,
      max_edges: 1_000,
    },
  },
});

const ATTACHED_ARCHIVE_SHA256 =
  "d714b63370b0026a28281f605794520fd4d1bc388ae8e5fdd367c5152cb95f6b";

function canonicalAttachedAssets() {
  const archive = new Uint8Array(2_048);
  const writeAscii = (offset, value) => {
    archive.set(Buffer.from(value, "ascii"), offset);
  };
  writeAscii(0, "data/train.csv");
  writeAscii(100, "0000644\0");
  writeAscii(108, "0000000\0");
  writeAscii(116, "0000000\0");
  writeAscii(124, "00000000003\0");
  writeAscii(136, "00000000000\0");
  writeAscii(148, "        ");
  writeAscii(156, "0");
  writeAscii(257, "ustar\0");
  writeAscii(263, "00");
  const checksum = archive
    .subarray(0, 512)
    .reduce((total, byte) => total + byte, 0);
  writeAscii(148, `${checksum.toString(8).padStart(6, "0")}\0 `);
  archive.set(Buffer.from("cat"), 512);
  return archive;
}

test("projection archive matches the shared Rust/Python bytes", () => {
  const dataset = Dataset.parse(
    "@prefix ex: <https://example.org/> .\nex:s ex:p ex:o .\n",
    "turtle",
  );
  const first = dataset.project("lpg-csv", CONFIG);
  const second = dataset.project("lpg-csv", CONFIG);

  assert.equal(first.profile, "lpg-csv");
  assert.deepEqual(first.archive, second.archive);
  assert.equal(
    createHash("sha256").update(first.archive).digest("hex"),
    "656066450fa23c55976f5434840169452c36324b943435e2f7ae55f8e9b6ef4e",
  );
  const ledger = JSON.parse(first.lossLedgerJson);
  assert.equal(ledger.schema_version, 1);
  assert.ok(ledger.losses.some((loss) => loss.code === "lpg-edge-semantics-lowered"));

  const lifted = liftProjection(first.archive, "lpg-csv", CONFIG);
  assert.equal(JSON.parse(lifted.lossLedgerJson).schema_version, 1);
  const liftedDataset = lifted.takeDataset();
  assert.equal(liftedDataset.size, 1);
  assert.equal(lifted.takeDataset(), undefined);
});

test("write-only profiles and mismatched tagged config fail explicitly", () => {
  const dataset = Dataset.parse(
    "<https://example.org/s> <https://example.org/p> <https://example.org/o> .\n",
    "ntriples",
  );
  assert.throws(() => dataset.project("graphml", CONFIG), /does not match/);
  assert.throws(
    () => liftProjection(new Uint8Array(), "skos", CONFIG),
    /not a bidirectional/,
  );
});

test("all research-object profiles execute through the shared WASM carrier", async () => {
  const fixture = (name) => new URL(
    `../../../rdf/tests/fixtures/research-objects/carrier/${name}`,
    import.meta.url,
  );
  const source = await readFile(fixture("shared.ttl"), "utf8");
  for (const profile of [
    "croissant-1.1",
    "ro-crate-1.3",
    "datacite-4.6",
    "dcat-3",
    "frictionless-data-package-1",
  ]) {
    const config = await readFile(fixture(`${profile}.json`), "utf8");
    const dataset = Dataset.parse(source, "turtle");
    const first = dataset.project(profile, config);
    const second = dataset.project(profile, config);
    assert.equal(first.profile, profile);
    assert.deepEqual(first.archive, second.archive);
    const lifted = liftProjection(first.archive, profile, config);
    assert.ok(lifted.takeDataset().size > 0);
  }
});

test("attached RO-Crate matches the cross-host archive through generated WASM", async () => {
  const fixture = (name) => new URL(
    `../../../rdf/tests/fixtures/research-objects/carrier/${name}`,
    import.meta.url,
  );
  const source = (await readFile(fixture("shared.ttl"), "utf8"))
    .replaceAll("files/train.csv", "data/train.csv")
    .replace(
      '"42"^^<https://example.org/rdf/role-50>',
      '"3"^^<https://example.org/rdf/role-50>',
    );
  const config = JSON.parse(await readFile(fixture("ro-crate-1.3.json"), "utf8"));
  config.config.packaging = "attached";
  const configJson = JSON.stringify(config);
  const assets = canonicalAttachedAssets();
  const dataset = Dataset.parse(source, "turtle");
  const first = dataset.projectWithAssets("ro-crate-1.3", configJson, assets);
  const second = dataset.projectWithAssets("ro-crate-1.3", configJson, assets);

  assert.deepEqual(first.archive, second.archive);
  assert.equal(
    createHash("sha256").update(first.archive).digest("hex"),
    ATTACHED_ARCHIVE_SHA256,
  );
  assert.equal(JSON.parse(first.lossLedgerJson).schema_version, 1);
  const lifted = liftProjection(first.archive, "ro-crate-1.3", configJson);
  assert.ok(lifted.takeDataset().size > 0);
});
