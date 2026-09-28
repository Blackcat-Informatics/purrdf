// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Child process for the poisoned-endpoint test of `handleSparqlRequest`. Poisoning is
// permanent for the JavaScript realm, so it runs here rather than in the parent test.
//
// The trap is `__purrdf_test_trap`, the instance's raw test export, armed by
// `globalThis.__purrdfArmTestTrap` exactly as `sync-trap-child.mjs` arms it. Before it,
// the valid neighbour: the endpoint answers a query with `200`. A second request is in
// flight across the trap, suspended on a `SERVICE` its resolver has been asked for. After
// the trap every call into the instance traps at the poison gate — the protocol parse,
// and `SparqlProtocolRequest.problemFor` too — and every request must still be answered
// with the sanitized `500` problem document and its correlation id, each request's error
// reported to `onInternalError` exactly once under that id.
//
// Prints one JSON line; the parent test asserts on it.

import { Dataset, QueryEngine, ServiceCatalog, ready } from "../../index.mjs";
import init from "../../pkg/purrdf_wasm.js";
import { handleSparqlRequest } from "../../cloudflare.mjs";

await ready();
const raw = await init();

const EX = "http://example.org/";
const REMOTE = "https://remote.example.org/sparql";
const BASE_URL = "https://endpoint.example.org/sparql";
const SHALLOW = `SELECT ?s WHERE { ?s <${EX}p> ?o }`;
const FEDERATED = `SELECT ?s ?x WHERE { ?s <${EX}p> ?o . SERVICE <${REMOTE}> { ?o <${EX}q> ?x } }`;

const errors = [];
const catalog = new ServiceCatalog();
catalog.addService(REMOTE, JSON.stringify({ capabilities: ["query", "network"] }));
let serviceAsked;
const asked = new Promise((resolve) => {
  serviceAsked = resolve;
});
const options = {
  engine: new QueryEngine(),
  dataset: Dataset.parse(`<${EX}a> <${EX}p> <${EX}o> .\n`, "nquads"),
  governors: { deadlineMs: 600_000 },
  catalog,
  // The host never answers: the request stays suspended until the poisoning rejects it.
  resolveService: () => {
    serviceAsked();
    return new Promise(() => {});
  },
  onInternalError: (error, context) => errors.push({ error, correlationId: context.correlationId }),
};
const get = (query) =>
  new Request(`${BASE_URL}?query=${encodeURIComponent(query)}`, {
    headers: { Accept: "application/sparql-results+json" },
  });

/** Answer `request`, and describe what the endpoint did: its response, or its escape. */
async function answer(request) {
  let response;
  try {
    response = await handleSparqlRequest(request, options);
  } catch (error) {
    return { settled: "threw", name: error?.name, message: String(error?.message) };
  }
  const text = await response.text();
  let body;
  try {
    body = JSON.parse(text);
  } catch {
    body = text;
  }
  const id = body?.correlationId;
  const reported = errors.filter((entry) => entry.correlationId === id);
  return {
    settled: "responded",
    status: response.status,
    contentType: response.headers.get("Content-Type"),
    vary: response.headers.get("Vary"),
    body,
    reported: reported.length,
    reportedName: reported[0]?.error?.name,
    reportedMessage: reported[0]?.error?.message,
  };
}

const report = {};
report.before = await answer(get(SHALLOW));
report.errorsBefore = errors.length;

const inFlight = answer(get(FEDERATED));
await asked;

globalThis.__purrdfArmTestTrap = true;
try {
  raw.__purrdf_test_trap();
  report.trapped = "returned";
} catch (error) {
  report.trapped = error?.name;
}

report.first = await answer(get(SHALLOW));
report.second = await answer(get(SHALLOW));
report.inFlight = await inFlight;
report.errorsAfter = errors.length;

process.stdout.write(`${JSON.stringify(report)}\n`);
// The in-flight resolver's promise never settles; nothing else is pending.
process.exit(0);
