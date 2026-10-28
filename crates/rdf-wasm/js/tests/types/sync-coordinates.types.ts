// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: the TypeScript compiler checks the shipped package's coordinate
// signatures across query and UPDATE options.
import { Dataset, QueryEngine, CompiledJsonLdContext, type QueryOptions } from "@blackcatinformatics/purrdf";
declare const engine: QueryEngine;
declare const dataset: Dataset;
declare const context: CompiledJsonLdContext;
const coordinates: QueryOptions = { now: "2000-01-01T00:00:00Z", entropySeed: 18446744073709551615n };
engine.query(dataset, "SELECT * {}", coordinates);
engine.select(dataset, "SELECT * {}", coordinates);
engine.ask(dataset, "ASK {}", coordinates).boolean;
engine.construct(dataset, "CONSTRUCT {} WHERE {}", coordinates).dataset;
engine.describe(dataset, "DESCRIBE <https://example.org/s>", coordinates).dataset;
engine.queryRaw(dataset, "SELECT * {}", coordinates).body;
engine.queryGoverned(dataset, "SELECT * {}", coordinates);
engine.queryEntailmentGoverned(dataset, "SELECT * {}", "rdfs", coordinates);
engine.explainQuery(dataset, "SELECT * {}", coordinates).body;
engine.queryRawConfigured(dataset, "CONSTRUCT {} WHERE {}", undefined, "jsonld", "{}", undefined, coordinates.now, coordinates.entropySeed).body;
engine.queryRawWithContext(dataset, "CONSTRUCT {} WHERE {}", undefined, "jsonld", context, undefined, undefined, coordinates.now, coordinates.entropySeed).body;
dataset.query("SELECT * {}", undefined, undefined, coordinates.now, coordinates.entropySeed).body;
engine.update(dataset, "INSERT DATA {}", coordinates).dataset;
engine.updateGoverned(dataset, "INSERT DATA {}", coordinates);
engine.updateAsync(dataset, "INSERT DATA {}", coordinates).then(result => result.dataset);
engine.updateGovernedAsync(dataset, "INSERT DATA {}", coordinates);
engine.update(dataset, "INSERT DATA {}", { now: "2000-01-01T00:00:00Z" }).dataset;
engine.updateGoverned(dataset, "INSERT DATA {}", { entropySeed: 0n });
engine.updateAsync(dataset, "INSERT DATA {}", { entropySeed: 0n }).then(result => result.dataset);
// @ts-expect-error Seeds cannot be supplied as strings.
dataset.query("SELECT * {}", undefined, undefined, undefined, "7").body;
