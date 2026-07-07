// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// The wasm-bindgen-generated class declarations (DataFactory/Dataset/Quad/Sink/Term)
// and the free functions (`version()`, `shaclValidateToSarif`, `shaclEntail`) are the
// source of truth for the engine surface — the whole `#[wasm_bindgen]` surface is
// re-exported from the package root (Dataset.canonicalize()/isomorphic() ship on the
// Dataset class itself).
export {
  DataFactory,
  Dataset,
  Quad,
  shaclEntail,
  shaclValidateToSarif,
  Sink,
  Term,
  version,
} from "./pkg/purrdf_wasm.js";

import type { Dataset, Quad } from "./pkg/purrdf_wasm.js";

/**
 * Instantiate the wasm module. Idempotent; await once before using any other API.
 * In Node the wasm bytes load from the colocated file automatically; in a browser,
 * pass the bytes/URL or omit to fetch the colocated `.wasm`.
 *
 * After `ready()`, the RDF/JS surface augmentations are live: `Dataset` is iterable
 * (`for (const quad of dataset)`); `Dataset.add`/`Dataset.delete` return the dataset
 * instance so calls chain (`ds.add(q1).add(q2)`); `Term.equals`/`Quad.equals` return
 * `false` for `null`/`undefined` instead of throwing; and
 * `DataFactory.literal(value, languageOrDatatype)` accepts a `NamedNode` datatype as
 * the RDF/JS spec allows (dispatching to `typedLiteral`).
 */
export function ready(wasmBytesOrUrl?: BufferSource | URL | string): Promise<void>;

/** An RDF/JS Stream (async iterable) of the dataset's quads. */
export function datasetToStream(dataset: Dataset): AsyncIterableIterator<Quad>;

/** Consume an (async) iterable of quads into a new Dataset via the engine's Sink. */
export function streamToDataset(
  quadStream: AsyncIterable<Quad> | Iterable<Quad>,
): Promise<Dataset>;
