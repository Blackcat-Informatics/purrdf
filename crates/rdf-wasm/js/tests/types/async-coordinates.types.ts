// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: only the TypeScript compiler can verify the shipped JavaScript option
// declarations accept async query coordinates and reject unsupported host entry points.
import { Dataset, QueryEngine, type AsyncQueryOptions } from "@blackcatinformatics/purrdf";
import { handleSparqlRequest } from "@blackcatinformatics/purrdf/cloudflare";

declare const engine: QueryEngine;
declare const dataset: Dataset;
const coordinates: AsyncQueryOptions = { now: "2000-01-01T00:00:00Z", entropySeed: 18446744073709551615n };
engine.queryAsync(dataset, "SELECT * WHERE {}", coordinates);
engine.queryRawBytesAsync(dataset, "SELECT * WHERE {}", coordinates).then(result => result.body);
engine.queryGovernedAsync(dataset, "SELECT * WHERE {}", coordinates);
engine.queryGovernedNegotiatedAsync(dataset, "SELECT * WHERE {}", coordinates);
engine.queryEntailmentGovernedAsync(dataset, "SELECT * WHERE {}", "rdfs", coordinates);
engine.explainQueryAsync(dataset, "SELECT * WHERE {}", coordinates).then(result => result.body);
dataset.queryAsync("SELECT * WHERE {}", coordinates).then(result => result.body);
handleSparqlRequest(new Request("https://example.org/query"), { engine, dataset, governors: { deadlineMs: 1000 }, ...coordinates });
engine.query(dataset, "SELECT * WHERE {}", { entropySeed: 0n });
engine.updateGovernedAsync(dataset, "INSERT DATA {}", { now: "2000-01-01T00:00:00Z" });
// @ts-expect-error Seed strings are not a checked JavaScript integer interface.
engine.queryAsync(dataset, "SELECT * WHERE {}", { entropySeed: "7" });
