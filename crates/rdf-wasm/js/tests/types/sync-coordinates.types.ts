// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: the TypeScript compiler checks the shipped package's coordinate
// signatures and the separation between query and UPDATE options.
import { Dataset, QueryEngine, CompiledJsonLdContext, type QueryOptions } from "@blackcatinformatics/purrdf";
declare const engine: QueryEngine;
declare const dataset: Dataset;
declare const context: CompiledJsonLdContext;
const coordinates: QueryOptions = { now: "2000-01-01T00:00:00Z", entropySeed: 18446744073709551615n };
engine.query(dataset, "SELECT * {}", coordinates);
engine.select(dataset, "SELECT * {}", coordinates);
engine.ask(dataset, "ASK {}", coordinates);
engine.construct(dataset, "CONSTRUCT {} WHERE {}", coordinates);
engine.describe(dataset, "DESCRIBE <https://example.org/s>", coordinates);
engine.queryRaw(dataset, "SELECT * {}", coordinates);
engine.queryGoverned(dataset, "SELECT * {}", coordinates);
engine.queryEntailmentGoverned(dataset, "SELECT * {}", "rdfs", coordinates);
engine.explainQuery(dataset, "SELECT * {}", coordinates);
engine.queryRawConfigured(dataset, "CONSTRUCT {} WHERE {}", undefined, "jsonld", "{}", undefined, coordinates.now, coordinates.entropySeed);
engine.queryRawWithContext(dataset, "CONSTRUCT {} WHERE {}", undefined, "jsonld", context, undefined, undefined, coordinates.now, coordinates.entropySeed);
dataset.query("SELECT * {}", undefined, undefined, coordinates.now, coordinates.entropySeed);
// @ts-expect-error UPDATE does not support query coordinates.
engine.update(dataset, "INSERT DATA {}", { now: "2000-01-01T00:00:00Z" });
// @ts-expect-error Governed UPDATE does not support query coordinates.
engine.updateGoverned(dataset, "INSERT DATA {}", { entropySeed: 0n });
// @ts-expect-error Async UPDATE remains outside this query interface.
engine.updateAsync(dataset, "INSERT DATA {}", { entropySeed: 0n });
// @ts-expect-error Seeds cannot be supplied as strings.
dataset.query("SELECT * {}", undefined, undefined, undefined, "7");
