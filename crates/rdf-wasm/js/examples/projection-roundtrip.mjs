// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

import { Dataset, liftProjection, ready } from "../index.mjs";

await ready();

const output = resolve(process.argv[2] ?? "projection.tar");
const config = JSON.stringify({
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
const dataset = Dataset.parse(
  "@prefix ex: <https://example.org/> . ex:alice ex:knows ex:bob .\n",
  "turtle",
);
const packageResult = dataset.project("lpg-csv", config);
await mkdir(resolve(output, ".."), { recursive: true });
await writeFile(output, packageResult.archive);

const lifted = liftProjection(packageResult.archive, "lpg-csv", config);
const liftedDataset = lifted.takeDataset();
if (liftedDataset?.size !== 1) {
  throw new Error("projection round trip changed the RDF dataset");
}
const ledger = JSON.parse(packageResult.lossLedgerJson);
console.log(`wrote ${packageResult.archive.length} bytes with ${ledger.losses.length} loss record(s)`);
