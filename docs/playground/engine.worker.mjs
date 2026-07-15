// SPDX-License-Identifier: MIT OR Apache-2.0
//
// The PurRDF console engine worker. This module OWNS the parsed Dataset state;
// the main thread is a pure view/message client. Every handler is wrapped in
// try/catch and posts { id, ok:false, error } on failure — engine errors are
// NEVER swallowed and NEVER become a silent empty result.

import {
  ready,
  DataFactory,
  Dataset,
  shaclValidateToSarif,
  shaclEntail,
  version,
} from "./purrdf/index.mjs";

const XSD_STRING = "http://www.w3.org/2001/XMLSchema#string";

/** The 6 codecs that can PARSE (JSON-LD is a first-class bidirectional codec). */
const PARSE_FORMATS = ["turtle", "ntriples", "nquads", "trig", "rdfxml", "jsonld"];
/** The 6 codecs that can SERIALIZE. */
const SERIALIZE_FORMATS = [
  "turtle",
  "ntriples",
  "nquads",
  "trig",
  "rdfxml",
  "jsonld",
];

/** @type {Dataset|null} The single source of truth for the console's graph. */
let current = null;

/**
 * @type {DataFactory|null} Lazily constructed AFTER the wasm is instantiated.
 * Constructing a `DataFactory` calls into the wasm module, so it must not run at
 * module-eval time (before `ready()` resolves) — every handler awaits `bootReady`
 * then `factory` is guaranteed live.
 */
let factory = null;

/** Ready gate — one `await ready()` before any engine call is used. */
const bootReady = ready();

/** Serialize a Term into a structured-clone-safe descriptor. */
function termToDescriptor(term) {
  if (term == null) return null;
  const t = { termType: term.termType, value: term.value };
  if (term.termType === "Literal") {
    t.language = term.language || "";
    t.direction = term.direction || "";
    const dt = term.datatype;
    t.datatype = dt ? dt.value : XSD_STRING;
  } else if (term.termType === "Quad") {
    // A quoted-triple term (RDF-1.2 wedge).
    t.subject = termToDescriptor(term.subject);
    t.predicate = termToDescriptor(term.predicate);
    t.object = termToDescriptor(term.object);
  }
  return t;
}

/** Rebuild a Term from a descriptor (used by match() filtering). */
function termFromDescriptor(d) {
  if (d == null) return undefined;
  switch (d.termType) {
    case "NamedNode":
      return factory.namedNode(d.value);
    case "BlankNode":
      return factory.blankNode(d.value);
    case "DefaultGraph":
      return factory.defaultGraph();
    case "Literal": {
      if (d.direction) {
        return factory.directionalLiteral(d.value, d.language || "", d.direction);
      }
      if (d.language) {
        return factory.literal(d.value, d.language);
      }
      if (d.datatype && d.datatype !== XSD_STRING) {
        return factory.typedLiteral(d.value, factory.namedNode(d.datatype));
      }
      return factory.literal(d.value);
    }
    case "Quad":
      return factory.quotedTriple(
        termFromDescriptor(d.subject),
        termFromDescriptor(d.predicate),
        termFromDescriptor(d.object),
      );
    default:
      throw new Error(`unknown termType: ${d.termType}`);
  }
}

/** Turn a Dataset's quads into an array of row descriptors. */
function quadsToRows(dataset) {
  return dataset.quads().map((q) => ({
    subject: termToDescriptor(q.subject),
    predicate: termToDescriptor(q.predicate),
    object: termToDescriptor(q.object),
    graph: termToDescriptor(q.graph),
  }));
}

/** Whether any quad has a quoted-triple (Quad) term in object position. */
function hasQuotedObject(dataset) {
  return dataset.quads().some((q) => q.object.termType === "Quad");
}

function requireCurrent() {
  if (current == null) {
    throw new Error("no graph parsed yet — load or parse an input document first");
  }
  return current;
}

const HANDLERS = {
  version() {
    return { version: version() };
  },

  parse({ text, format }) {
    if (!PARSE_FORMATS.includes(format)) {
      throw new Error(
        `unknown parse format "${format}" — expected one of ${PARSE_FORMATS.join(", ")}`,
      );
    }
    current = Dataset.parse(text, format);
    return { size: current.size, rows: quadsToRows(current) };
  },

  serializeAll() {
    const ds = requireCurrent();
    const quoted = hasQuotedObject(ds);
    const formats = {};
    for (const f of SERIALIZE_FORMATS) {
      const entry = {};
      // A serializer can HARD-FAIL for a given graph (a format that cannot
      // losslessly encode a construct surfaces the error). Surface that per-format
      // error visibly instead of aborting the whole differential.
      let text = null;
      try {
        text = ds.serialize(f);
      } catch (e) {
        entry.error = String(e?.message ?? e);
      }
      entry.text = text;
      if (text != null) {
        // Honest round-trip fidelity: re-parse and confirm graph identity.
        try {
          const back = Dataset.parse(text, f);
          entry.roundtrips = ds.isomorphic(back);
        } catch {
          entry.roundtrips = false;
        }
      } else {
        entry.roundtrips = false;
      }
      formats[f] = entry;
    }
    return {
      formats,
      canonical: ds.canonicalize(),
      hasQuotedObject: quoted,
      size: ds.size,
    };
  },

  query({ sparql }) {
    const ds = requireCurrent();
    const out = ds.query(sparql); // throws on SERVICE/LOAD/parse/eval error
    let parsed = null;
    try {
      parsed = JSON.parse(out);
    } catch {
      parsed = null;
    }
    if (parsed && (Object.hasOwn(parsed, "head") || Object.hasOwn(parsed, "boolean"))) {
      return { kind: "srj", srj: parsed };
    }
    // CONSTRUCT / DESCRIBE → Turtle text.
    return { kind: "turtle", turtle: out };
  },

  shaclValidate({ shapes }) {
    const ds = requireCurrent();
    const dataNt = ds.serialize("ntriples");
    const sarif = JSON.parse(shaclValidateToSarif(shapes, dataNt));
    return { sarif };
  },

  shaclEntail({ shapes }) {
    const ds = requireCurrent();
    const dataNt = ds.serialize("ntriples");
    const entailed = shaclEntail(shapes, dataNt);
    return { ntriples: entailed };
  },

  identity({ aText, aFormat, bText, bFormat }) {
    if (!PARSE_FORMATS.includes(aFormat)) {
      throw new Error(`unknown parse format "${aFormat}" for graph A`);
    }
    if (!PARSE_FORMATS.includes(bFormat)) {
      throw new Error(`unknown parse format "${bFormat}" for graph B`);
    }
    const a = Dataset.parse(aText, aFormat);
    const b = Dataset.parse(bText, bFormat);
    return {
      isomorphic: a.isomorphic(b),
      canonicalA: a.canonicalize(),
      canonicalB: b.canonicalize(),
    };
  },

  quads() {
    const ds = requireCurrent();
    return { rows: quadsToRows(ds), size: ds.size };
  },

  match({ subject, predicate, object, graph }) {
    const ds = requireCurrent();
    const filtered = ds.match(
      termFromDescriptor(subject),
      termFromDescriptor(predicate),
      termFromDescriptor(object),
      termFromDescriptor(graph),
    );
    return { rows: quadsToRows(filtered), size: filtered.size };
  },
};

self.addEventListener("message", async (event) => {
  // Defence-in-depth: a dedicated worker only ever receives messages from the
  // document that spawned it (event.origin is the empty string, never a foreign
  // origin), so reject anything tagged with a cross-origin source.
  if (event.origin && event.origin !== self.location.origin) return;
  const { id, op, args } = event.data ?? {};
  try {
    await bootReady;
    // The wasm is now instantiated; build the factory once, on first use.
    if (factory === null) factory = new DataFactory();
    const handler = HANDLERS[op];
    if (!handler) throw new Error(`unknown op: ${op}`);
    const result = await handler(args ?? {});
    self.postMessage({ id, ok: true, result });
  } catch (e) {
    // HARD FAIL, surfaced — never swallow, never a silent empty result.
    self.postMessage({ id, ok: false, error: String(e?.message ?? e) });
  }
});
