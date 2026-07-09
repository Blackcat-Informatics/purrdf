// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// purrdf — the idiomatic RDF/JS surface over the wasm engine.
//
// The wasm-bindgen-generated classes (DataFactory/Dataset/Quad/Sink/Term) and the
// free functions (version, shaclValidateToSarif, shaclEntail) are re-exported as-is —
// the whole `#[wasm_bindgen]` surface is reachable from the package root, so
// SHACL validation/entailment and Dataset.canonicalize()/isomorphic() need no deep
// `./pkg/` import. This wrapper adds the isomorphic glue that the synchronous
// wasm boundary cannot express in Rust:
//   * `ready()` — one-time async wasm instantiation (required for the `web` target).
//   * the polymorphic RDF/JS `DataFactory.literal(value, languageOrDatatype)` —
//     dispatching a NamedNode datatype argument to `typedLiteral` and a
//     `{ language, direction }` argument to `directionalLiteral`.
//   * `Dataset` iterability (`for (const quad of dataset)`).
//   * `Dataset.from`, `Dataset#toStream`, and `DataFactory#dataset`.
//   * `datasetToStream` / `streamToDataset` — the async RDF/JS Stream/Sink primitives
//     over the synchronous `Dataset.quads()` / `Sink` engine surface.

import init, {
  DataFactory,
  Dataset,
  Quad,
  QueryEngine,
  shaclEntail,
  shaclValidateToSarif,
  Sink,
  Term,
  version,
} from "./pkg/purrdf_wasm.js";

let _ready = false;

function isNamedNodeTerm(value) {
  return (
    value != null &&
    typeof value === "object" &&
    value.termType === "NamedNode"
  );
}

function isDirectionalLanguage(value) {
  return (
    value != null &&
    typeof value === "object" &&
    typeof value.language === "string" &&
    (value.direction === "ltr" || value.direction === "rtl")
  );
}

function normalizeQueryOptions(options) {
  if (options == null) return { base: undefined, format: undefined };
  if (typeof options !== "object") {
    throw new TypeError("query options must be an object when supplied");
  }
  return {
    base: options.base ?? undefined,
    format: options.format ?? undefined,
  };
}

function selectResultToObject(raw) {
  const variables = raw.variables;
  const rawRows = raw.rows;
  try {
    const rows = rawRows.map((row) => {
      const out = Object.create(null);
      for (const variable of variables) {
        const value = row.get(variable);
        if (value !== undefined) out[variable] = value;
      }
      return out;
    });
    return { kind: "select", variables, rows };
  } finally {
    for (const row of rawRows) row.free?.();
    raw.free?.();
  }
}

function queryResultToObject(raw) {
  try {
    switch (raw.kind) {
      case "select": {
        const select = raw.takeSelect();
        if (select === undefined) throw new Error("SELECT result was already consumed");
        return selectResultToObject(select);
      }
      case "ask":
        return { kind: "ask", boolean: raw.boolean };
      case "graph": {
        const dataset = raw.takeDataset();
        if (dataset === undefined) throw new Error("graph result was already consumed");
        return { kind: "graph", dataset };
      }
      default:
        throw new Error(`unknown SPARQL result kind ${raw.kind}`);
    }
  } finally {
    raw.free?.();
  }
}

/**
 * Instantiate the wasm module. Idempotent. In Node the wasm bytes are read from the
 * colocated file; in a browser, pass the bytes/URL (or omit to fetch the colocated
 * `.wasm`). Must be awaited once before any other API is used.
 */
export async function ready(wasmBytesOrUrl) {
  if (_ready) return;
  if (wasmBytesOrUrl !== undefined) {
    await init({ module_or_path: wasmBytesOrUrl });
  } else if (typeof process !== "undefined" && process.versions?.node) {
    const { readFile } = await import("node:fs/promises");
    const { fileURLToPath } = await import("node:url");
    const wasmPath = fileURLToPath(
      new URL("./pkg/purrdf_wasm_bg.wasm", import.meta.url),
    );
    await init({ module_or_path: await readFile(wasmPath) });
  } else {
    await init();
  }

  // RDF/JS DatasetCore is iterable over its quads.
  if (!Dataset.prototype[Symbol.iterator]) {
    Dataset.prototype[Symbol.iterator] = function () {
      return this.quads()[Symbol.iterator]();
    };
  }

  if (!Dataset.from) {
    Dataset.from = function (quads = []) {
      const dataset = new Dataset();
      for (const quad of quads) dataset.add(quad);
      return dataset;
    };
  }

  if (!Dataset.prototype.toStream) {
    Dataset.prototype.toStream = function () {
      return datasetToStream(this);
    };
  }

  // RDF/JS DatasetCore.add(quad)/delete(quad) MUST return the dataset instance so calls
  // chain (`ds.add(q1).add(q2)`). The wasm methods return a bool ("did the effective set
  // change?"); the spec surface returns `this` (the changed-bit stays observable via
  // `size`). The guard applied here is the same boundary the equals/literal shims use.
  for (const method of ["add", "delete"]) {
    const flag = `__purrdfChaining_${method}`;
    if (!Dataset.prototype[flag]) {
      const wasmMutate = Dataset.prototype[method];
      Dataset.prototype[method] = function (quad) {
        wasmMutate.call(this, quad);
        return this;
      };
      Dataset.prototype[flag] = true;
    }
  }

  // RDF/JS spec: Term.equals(other) / Quad.equals(other) MUST return false when `other`
  // is null or undefined — "Returns false if other is undefined or null." The wasm
  // `equals` takes a borrowed `&Term`/`&Quad` (non-consuming — the argument stays usable
  // afterwards), but wasm-bindgen throws on a null borrow, so the null/undefined guard is
  // applied here, one layer out (the same boundary where the polymorphic literal() lives).
  for (const Klass of [Term, Quad]) {
    if (!Klass.prototype.__purrdfNullSafeEquals) {
      const wasmEquals = Klass.prototype.equals;
      Klass.prototype.equals = function (other) {
        if (other === null || other === undefined) return false;
        return wasmEquals.call(this, other);
      };
      Klass.prototype.__purrdfNullSafeEquals = true;
    }
  }

  // Present the RDF/JS-spec polymorphic literal(value, languageOrDatatype). The wasm
  // method takes `(value, language?)`; a NamedNode second argument is a datatype.
  // PurRDF also accepts `{ language, direction }` for RDF 1.2 dirLangString literals.
  if (!DataFactory.prototype.__purrdfPolymorphicLiteral) {
    const wasmLiteral = DataFactory.prototype.literal;
    DataFactory.prototype.literal = function (value, languageOrDatatype) {
      if (isNamedNodeTerm(languageOrDatatype)) {
        return this.typedLiteral(value, languageOrDatatype);
      }
      if (isDirectionalLanguage(languageOrDatatype)) return this.directionalLiteral(value, languageOrDatatype.language, languageOrDatatype.direction);
      return wasmLiteral.call(this, value, languageOrDatatype ?? undefined);
    };
    DataFactory.prototype.__purrdfPolymorphicLiteral = true;
  }

  if (!DataFactory.prototype.dataset) {
    DataFactory.prototype.dataset = function (quads = []) {
      return Dataset.from(quads);
    };
  }

  if (!QueryEngine.prototype.__purrdfPackageRootApi) {
    const wasmQuery = QueryEngine.prototype.query;
    const wasmSelect = QueryEngine.prototype.select;
    const wasmAsk = QueryEngine.prototype.ask;
    const wasmConstruct = QueryEngine.prototype.construct;
    const wasmDescribe = QueryEngine.prototype.describe;
    const wasmUpdate = QueryEngine.prototype.update;
    const wasmQueryRaw = QueryEngine.prototype.queryRaw;

    QueryEngine.prototype.query = function (dataset, sparql, options) {
      const { base } = normalizeQueryOptions(options);
      return queryResultToObject(wasmQuery.call(this, dataset, sparql, base));
    };
    QueryEngine.prototype.select = function (dataset, sparql, options) {
      const { base } = normalizeQueryOptions(options);
      return selectResultToObject(wasmSelect.call(this, dataset, sparql, base));
    };
    QueryEngine.prototype.ask = function (dataset, sparql, options) {
      const { base } = normalizeQueryOptions(options);
      return wasmAsk.call(this, dataset, sparql, base);
    };
    QueryEngine.prototype.construct = function (dataset, sparql, options) {
      const { base } = normalizeQueryOptions(options);
      return wasmConstruct.call(this, dataset, sparql, base);
    };
    QueryEngine.prototype.describe = function (dataset, sparql, options) {
      const { base } = normalizeQueryOptions(options);
      return wasmDescribe.call(this, dataset, sparql, base);
    };
    QueryEngine.prototype.update = function (dataset, sparql, options) {
      const { base } = normalizeQueryOptions(options);
      wasmUpdate.call(this, dataset, sparql, base);
      return dataset;
    };
    QueryEngine.prototype.queryRaw = function (dataset, sparql, options) {
      const { base, format } = normalizeQueryOptions(options);
      return wasmQueryRaw.call(this, dataset, sparql, base, format);
    };
    QueryEngine.prototype.__purrdfPackageRootApi = true;
  }

  _ready = true;
}

/**
 * An RDF/JS Stream of the dataset's quads — an async iterable. (The engine is
 * synchronous; the async wrapper is the RDF/JS Stream contract.)
 */
export function datasetToStream(dataset) {
  const quads = dataset.quads();
  return (async function* () {
    for (const quad of quads) yield quad;
  })();
}

/**
 * Consume an (async) iterable of quads into a new Dataset, via the engine's streaming
 * Sink (the purrdf-events ingestion protocol + its finish() resolution).
 */
export async function streamToDataset(quadStream) {
  const sink = new Sink();
  for await (const quad of quadStream) sink.push(quad);
  return sink.finish();
}

export {
  DataFactory,
  Dataset,
  Quad,
  QueryEngine,
  shaclEntail,
  shaclValidateToSarif,
  Sink,
  Term,
  version,
};
