// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Child process for the "without JSPI" test: run under `no-jspi-preload.mjs`. Prints one
// JSON line describing what the package did; the parent test asserts on it.

import { Dataset, QueryEngine, hasAsyncQueries, ready } from "../../index.mjs";
import { NO_JSPI_MESSAGE } from "../../pkg/purrdf_jspi.mjs";

await ready();

const EX = "http://example.org/";
const data = Dataset.parse(`<${EX}a> <${EX}p> <${EX}o> .\n`, "nquads");
const engine = new QueryEngine();
const report = {
  suspendingPresent: typeof WebAssembly.Suspending,
  promisingPresent: typeof WebAssembly.promising,
  hasAsyncQueries: hasAsyncQueries(),
  noJspiMessage: NO_JSPI_MESSAGE,
};

// The synchronous API is unaffected.
report.syncRows = engine.select(data, `SELECT ?s WHERE { ?s <${EX}p> ?o }`).rowCount;

async function rejection(promise) {
  try {
    await promise;
    return { settled: "resolved" };
  } catch (error) {
    return { settled: "rejected", name: error?.name, message: error?.message };
  }
}

// An ordinary asynchronous call is refused with the one clear message.
report.queryAsync = await rejection(engine.queryAsync(data, `SELECT ?s WHERE { ?s <${EX}p> ?o }`));
report.updateAsync = await rejection(engine.updateAsync(data, `INSERT DATA { <${EX}b> <${EX}p> <${EX}o> }`));
report.datasetQueryAsync = await rejection(data.queryAsync(`SELECT ?s WHERE { ?s <${EX}p> ?o }`));
// A call whose arguments wasm itself would refuse — a dataset that is not a Dataset —
// is still refused for the missing JSPI: the check runs before anything touches wasm.
report.beforeWasm = await rejection(engine.queryAsync({ notADataset: true }, "SELECT * WHERE { ?s ?p ?o }"));
// The refused update left the dataset alone.
report.sizeAfter = data.size;

process.stdout.write(`${JSON.stringify(report)}\n`);
