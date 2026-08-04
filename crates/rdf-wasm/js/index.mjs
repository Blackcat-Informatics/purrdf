// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// purrdf — the idiomatic RDF/JS surface over the wasm engine.
//
// The wasm-bindgen-generated classes (DataFactory/Dataset/Quad/Sink/Term,
// RegimeClosure, ReasoningAnswer) and the free functions (version,
// shaclValidateToSarif, shaclEntail, entailMaterialize, entailRules,
// entailImplementedRules, entailCheckGoldenVectors,
// entailCheckInconsistentRefusal, entailConsistency, entailClassify,
// entailRealize, entailInstances, entailEntails, entailProfile,
// entailExtensions, entailExtractModule, entailJustify,
// entailExplainConclusion) are re-exported
// as-is — the whole `#[wasm_bindgen]` surface is reachable from the package
// root, so SHACL validation/entailment, the DL reasoning services, and
// Dataset.canonicalize()/isomorphic() need no deep `./pkg/` import. This
// wrapper adds the isomorphic glue that the synchronous wasm boundary cannot
// express in Rust:
//   * `ready()` — one-time async wasm instantiation (required for the `web` target).
//   * the polymorphic RDF/JS `DataFactory.literal(value, languageOrDatatype)` —
//     dispatching a NamedNode datatype argument to `typedLiteral` and a
//     `{ language, direction }` argument to `directionalLiteral`.
//   * `Dataset` iterability (`for (const quad of dataset)`).
//   * `Dataset.from`, `Dataset#toStream`, and `DataFactory#dataset`.
//   * `datasetToStream` / `streamToDataset` — the async RDF/JS Stream/Sink primitives
//     over the synchronous `Dataset.quads()` / `Sink` engine surface.
//   * the governed-outcome shape normalizers — `queryGoverned`/`updateGoverned` return
//     wasm-owned objects whose fields have to be MOVED out one at a time; the wrappers
//     below drain them into ordinary JS objects and free the handles, exactly as
//     `queryResultToObject` already does for an ungoverned result.
//
// What this module deliberately does NOT do is decide anything about governors. It sets
// no default ceiling, applies no fallback, and never converts a trip into a throw: the
// ceilings it reads off an options object go straight to Rust, which validates them and
// owns every policy (the metering base, the inclusive boundary, the precedence between
// two governors that fire at once). The only transformation applied on the way in is a
// `BigInt` coercion, because the boundary type is a 64-bit integer and JavaScript spells
// those `bigint` — a shape change, not a decision.

import init, {
  CancellationToken,
  CompiledJsonLdContext,
  DataFactory,
  Dataset,
  entailCertainAnswers,
  entailCheckGoldenVectors,
  entailCheckInconsistentRefusal,
  entailClassify,
  entailConsistency,
  entailEntails,
  entailExplainConclusion,
  entailExtensions,
  entailExtractModule,
  entailGraphEntails,
  entailImplementedRules,
  entailInstances,
  entailJustify,
  entailMaterialize,
  entailProfile,
  entailRealize,
  entailRules,
  entailVerifyEntailment,
  governorDimensions,
  liftProjection,
  ProjectionLift,
  ProjectionPackage,
  Quad,
  QueryEngine,
  ReasoningAnswer,
  RegimeClosure,
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

// A governor ceiling as the wasm boundary spells it: a 64-bit integer, i.e. a JS bigint.
// `undefined` means the caller declined the dimension and is passed through as such —
// this function invents no ceiling of its own. A negative value is NOT rejected here:
// Rust refuses it, so the one refusal message lives with the one owner of the rule.
function governorCeiling(value, name) {
  if (value === undefined || value === null) return undefined;
  try {
    return BigInt(value);
  } catch {
    throw new TypeError(
      `query option ${name} must be an integer (number, bigint, or integral string)`,
    );
  }
}

// The option keys that only the GOVERNED entry points enforce. Rust never sees a JS
// options object — it receives positional arguments — so an ungoverned call cannot
// possibly notice one of these and would silently run with no ceiling at all. That is the
// exact failure a governor must not have, and this is the only layer that can see the key,
// so the ungoverned normalizer refuses them by name instead of dropping them.
const GOVERNOR_OPTION_KEYS = [
  "fuel",
  "deadlineMs",
  "maxAnswers",
  "maxIntermediateCells",
  "maxScratchBytes",
  "maxRemoteRequests",
  "cancel",
];

function normalizeQueryOptions(options) {
  if (options == null) return { base: undefined, format: undefined };
  if (typeof options !== "object") {
    throw new TypeError("query options must be an object when supplied");
  }
  for (const key of GOVERNOR_OPTION_KEYS) {
    if (options[key] != null) {
      throw new TypeError(
        `query option ${key} is an execution governor and is enforced only by ` +
          `queryGoverned/updateGoverned; this call would ignore it entirely`,
      );
    }
  }
  return {
    base: options.base ?? undefined,
    format: options.format ?? undefined,
  };
}

function normalizeGovernedOptions(options) {
  if (options == null) return { base: undefined };
  if (typeof options !== "object") {
    throw new TypeError("query options must be an object when supplied");
  }
  return {
    base: options.base ?? undefined,
    fuel: governorCeiling(options.fuel, "fuel"),
    deadlineMs: governorCeiling(options.deadlineMs, "deadlineMs"),
    maxAnswers: governorCeiling(options.maxAnswers, "maxAnswers"),
    maxIntermediateCells: governorCeiling(
      options.maxIntermediateCells,
      "maxIntermediateCells",
    ),
    maxScratchBytes: governorCeiling(options.maxScratchBytes, "maxScratchBytes"),
    maxRemoteRequests: governorCeiling(options.maxRemoteRequests, "maxRemoteRequests"),
    // A governed call CONSUMES the token handle it is given (wasm-bindgen moves an owned
    // exported value), so the engine gets a share and the caller keeps their own token
    // usable across the whole sequence of calls it governs.
    cancel: options.cancel == null ? undefined : options.cancel.share(),
  };
}

function normalizeEntailmentGovernedOptions(options) {
  const governed = normalizeGovernedOptions(options);
  return {
    ...governed,
    program: options?.program ?? undefined,
  };
}

function visualizationOptionsJson(options) {
  if (options == null) return undefined;
  if (typeof options !== "object" || Array.isArray(options)) {
    throw new TypeError("visualization options must be an object when supplied");
  }
  return JSON.stringify(options);
}

function selectResultToObject(raw) {
  const variables = raw.variables;
  const length = raw.rowCount;
  let closed = false;

  const close = () => {
    if (closed) return;
    closed = true;
    raw.free?.();
  };
  if (length === 0) close();
  const materialize = (row) => {
    try {
      const out = Object.create(null);
      for (let index = 0; index < variables.length; index += 1) {
        const variable = variables[index];
        const value = row.takeValue(index);
        if (value !== undefined) out[variable] = value;
      }
      return out;
    } finally {
      row.free?.();
      if (raw.remaining === 0) close();
    }
  };
  const rows = {
    get length() {
      return length;
    },
    get remaining() {
      return closed ? 0 : raw.remaining;
    },
    take(index) {
      if (closed) return undefined;
      const row = raw.takeRow(index);
      return row === undefined ? undefined : materialize(row);
    },
    next() {
      if (closed) return { done: true, value: undefined };
      const row = raw.nextRow();
      if (row === undefined) {
        close();
        return { done: true, value: undefined };
      }
      return { done: false, value: materialize(row) };
    },
    return() {
      close();
      return { done: true, value: undefined };
    },
    toArray() {
      return Array.from(this);
    },
    free: close,
    [Symbol.iterator]() {
      return this;
    },
  };
  return { kind: "select", variables, rowCount: length, rows, free: close };
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

// The kernel's dimension vocabulary, read from the engine once and cached. It is NOT
// restated here: `governorDimensions()` is the engine's own declaration order, and it is
// the index order of the evidence vectors below, so a dimension the kernel adds shows up
// in every caller's evidence map without this file being touched.
let _governorDimensions;

function governorDimensionLabels() {
  _governorDimensions ??= governorDimensions();
  return _governorDimensions;
}

function governorEvidenceToObject(raw) {
  try {
    const labels = governorDimensionLabels();
    const consumed = raw.consumed;
    const limits = raw.limits;
    const consumedBy = Object.create(null);
    const limitsBy = Object.create(null);
    for (let index = 0; index < labels.length; index += 1) {
      consumedBy[labels[index]] = consumed[index];
      limitsBy[labels[index]] = limits[index];
    }
    return { isComplete: raw.isComplete, consumed: consumedBy, limits: limitsBy };
  } finally {
    raw.free?.();
  }
}

function trippedGovernorToObject(raw) {
  try {
    return {
      kind: raw.kind,
      label: raw.label,
      dimension: raw.dimension,
      limit: raw.limit,
      consumed: raw.consumed,
      estimate: raw.estimate,
      cause: raw.cause,
      message: raw.message,
    };
  } finally {
    raw.free?.();
  }
}

function partialAnswersToObject(raw) {
  try {
    const certainty = raw.certainty;
    const isCertain = raw.isCertain;
    const isPositionalPrefix = raw.isPositionalPrefix;
    const barrier = raw.barrier;
    const result = raw.takeResult();
    return {
      certainty,
      isCertain,
      isPositionalPrefix,
      barrier,
      result: result === undefined ? undefined : queryResultToObject(result),
    };
  } finally {
    raw.free?.();
  }
}

function queryOutcomeToObject(raw) {
  try {
    // `isComplete` is read before anything is moved out: the discriminator has to survive
    // the drain, and a trip is an OUTCOME here — nothing on this path throws.
    const isComplete = raw.isComplete;
    const result = raw.takeResult();
    const partial = raw.takePartial();
    const tripped = raw.takeTripped();
    const evidence = raw.takeEvidence();
    return {
      isComplete,
      result: result === undefined ? undefined : queryResultToObject(result),
      partial: partial === undefined ? undefined : partialAnswersToObject(partial),
      tripped: tripped === undefined ? undefined : trippedGovernorToObject(tripped),
      evidence: evidence === undefined ? undefined : governorEvidenceToObject(evidence),
    };
  } finally {
    raw.free?.();
  }
}

function entailmentQueryOutcomeToObject(raw) {
  try {
    const isComplete = raw.isComplete;
    const closureStopped = raw.closureStopped;
    const report = raw.report;
    const outcome = raw.takeOutcome();
    const tripped = raw.takeTripped();
    return {
      phase: closureStopped ? "closure-stopped" : "answered",
      isComplete,
      outcome: outcome === undefined ? undefined : queryOutcomeToObject(outcome),
      report,
      tripped: tripped === undefined ? undefined : trippedGovernorToObject(tripped),
    };
  } finally {
    raw.free?.();
  }
}

function updateOutcomeToObject(raw) {
  try {
    const isApplied = raw.isApplied;
    const tripped = raw.takeTripped();
    const evidence = raw.takeEvidence();
    return {
      isApplied,
      tripped: tripped === undefined ? undefined : trippedGovernorToObject(tripped),
      evidence: evidence === undefined ? undefined : governorEvidenceToObject(evidence),
    };
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
      for (const quad of quads ?? []) dataset.add(quad);
      return dataset;
    };
  }

  if (!Dataset.prototype.toStream) {
    Dataset.prototype.toStream = function () {
      return datasetToStream(this);
    };
  }

  if (!Dataset.prototype.__purrdfVisualizationApi) {
    const visualModelJson = Dataset.prototype.visualModelJson;
    const visualExportJson = Dataset.prototype.visualExportJson;
    const visualSvgJson = Dataset.prototype.visualSvgJson;
    Dataset.prototype.visualModel = function (options) {
      return JSON.parse(visualModelJson.call(this, visualizationOptionsJson(options)));
    };
    Dataset.prototype.visualExport = function (options) {
      return JSON.parse(visualExportJson.call(this, visualizationOptionsJson(options)));
    };
    Dataset.prototype.visualSvg = function (options) {
      return JSON.parse(visualSvgJson.call(this, visualizationOptionsJson(options)));
    };
    Dataset.prototype.__purrdfVisualizationApi = true;
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
      return Dataset.from(quads ?? []);
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
    const wasmQueryGoverned = QueryEngine.prototype.queryGoverned;
    const wasmQueryEntailmentGoverned = QueryEngine.prototype.queryEntailmentGoverned;
    const wasmUpdateGoverned = QueryEngine.prototype.updateGoverned;
    const wasmExplainQuery = QueryEngine.prototype.explainQuery;

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
    QueryEngine.prototype.queryGoverned = function (dataset, sparql, options) {
      const o = normalizeGovernedOptions(options);
      return queryOutcomeToObject(
        wasmQueryGoverned.call(
          this,
          dataset,
          sparql,
          o.base,
          o.fuel,
          o.deadlineMs,
          o.maxAnswers,
          o.maxIntermediateCells,
          o.maxScratchBytes,
          o.maxRemoteRequests,
          o.cancel,
        ),
      );
    };
    QueryEngine.prototype.queryEntailmentGoverned = function (
      dataset,
      sparql,
      entailment,
      options,
    ) {
      const o = normalizeEntailmentGovernedOptions(options);
      return entailmentQueryOutcomeToObject(
        wasmQueryEntailmentGoverned.call(
          this,
          dataset,
          sparql,
          o.base,
          entailment,
          o.program,
          o.fuel,
          o.deadlineMs,
          o.maxAnswers,
          o.maxIntermediateCells,
          o.maxScratchBytes,
          o.maxRemoteRequests,
          o.cancel,
        ),
      );
    };
    QueryEngine.prototype.updateGoverned = function (dataset, sparql, options) {
      // `maxAnswers` is forwarded rather than dropped: an UPDATE has no answer sequence to
      // bound, and Rust refuses it by name. Silently ignoring it here would be a ceiling
      // the caller believes they set.
      const o = normalizeGovernedOptions(options);
      return updateOutcomeToObject(
        wasmUpdateGoverned.call(
          this,
          dataset,
          sparql,
          o.base,
          o.fuel,
          o.deadlineMs,
          o.maxAnswers,
          o.maxIntermediateCells,
          o.maxScratchBytes,
          o.maxRemoteRequests,
          o.cancel,
        ),
      );
    };
    QueryEngine.prototype.explainQuery = function (dataset, sparql, options) {
      const { base } = normalizeQueryOptions(options);
      return wasmExplainQuery.call(this, dataset, sparql, base);
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
  CancellationToken,
  CompiledJsonLdContext,
  DataFactory,
  Dataset,
  entailCertainAnswers,
  entailCheckGoldenVectors,
  entailCheckInconsistentRefusal,
  entailClassify,
  entailConsistency,
  entailEntails,
  entailExplainConclusion,
  entailExtensions,
  entailExtractModule,
  entailGraphEntails,
  entailImplementedRules,
  entailInstances,
  entailJustify,
  entailMaterialize,
  entailProfile,
  entailRealize,
  entailRules,
  entailVerifyEntailment,
  governorDimensions,
  liftProjection,
  ProjectionLift,
  ProjectionPackage,
  Quad,
  QueryEngine,
  ReasoningAnswer,
  RegimeClosure,
  shaclEntail,
  shaclValidateToSarif,
  Sink,
  Term,
  version,
};
