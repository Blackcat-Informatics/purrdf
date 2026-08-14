<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# SPARQL: Result Formats

[`purrdf-sparql-results`](https://docs.rs/purrdf-sparql-results) is the results
boundary of the SPARQL stack: the canonical authority for turning a
`SparqlResult` (SELECT solutions, ASK boolean, or CONSTRUCT graph) into the
four W3C SPARQL Results formats — JSON (SRJ), XML, CSV, and TSV — plus an
additive, caller-named provenance extension where the format can carry one.
JSON and XML documents can also be read back (`from_json`, `from_xml`).

```rust,ignore
use purrdf::sparql::{serialize, ResultProvenance, SparqlResultsFormat};

// `result` is the SparqlResult produced by purrdf-sparql-eval (or any engine
// implementing the purrdf-core SparqlEngine seam). `None` here means "carry
// no provenance extension" — see "The provenance extension" below.
let outcome = serialize(&result, SparqlResultsFormat::Json, &ResultProvenance::default(), None)
    .expect("SELECT serializes to SRJ");

assert!(!outcome.provenance_dropped);
let json = String::from_utf8(outcome.bytes).unwrap();
```

Per-format writers (`to_json`, `to_xml`, `to_csv`, `to_tsv`) and readers
(`from_json`, `from_json_boolean`, `from_xml`, `from_xml_boolean`) are also
exported directly.

## Behavior worth knowing before you pick a format

- **Byte-deterministic output** — the same result always serializes to the
  same bytes, like every other PurRDF output path
  ([Codecs & Determinism](../concepts/codecs.md)).
- **The support matrix is enforced, not fudged** — XML rejects CONSTRUCT
  graphs, and CSV/TSV reject both ASK booleans and CONSTRUCT graphs, each as
  a typed `Error::Format`, rather than emitting something spec-shaped but
  wrong.
- **Lossy projections are flagged** — CSV/TSV have no extension point, so a
  populated provenance is trimmed at the exit gate and
  `SerializeOutcome::provenance_dropped` is set; the drop is never silent. The
  same flag is set for JSON/XML when a non-empty `ResultProvenance` is supplied
  with no `ProvenanceNamespace` to anchor it under — see below.
- **RDF 1.2 base direction uses the spec's own key** — a directional literal's
  base direction serializes as `its:dir` (JSON: an additive `"its:dir"` member
  beside `"value"`/`"xml:lang"`; XML: `<literal its:dir="…">`), the same
  spelling the SPARQL 1.2 Query Results specification's own example uses and
  the RDF/XML codec already emits. The XML/JSON readers also accept two
  legacy spellings this crate's own writer previously produced (a bare `dir`
  attribute, and — XML only — a `purrdf:dir`-namespaced one) for backward
  compatibility, but `its:dir` wins when more than one is present on the same
  literal, and only `its:dir` is ever written.

| Format | SELECT | ASK | CONSTRUCT | Provenance extension |
| --- | --- | --- | --- | --- |
| JSON (SRJ) | yes | yes | yes | yes, with a namespace |
| XML | yes | yes | rejected | yes, with a namespace |
| CSV | yes | rejected | rejected | dropped, flagged |
| TSV | yes | rejected | rejected | dropped, flagged |

## The provenance extension

The provenance extension is **additive** and **caller-named**: a standard
SPARQL results consumer can read the JSON/XML documents unchanged, while a
provenance-aware consumer can recover per-result data carried alongside the
bindings, anchored under an identifier the *caller* supplies — never a
purrdf-minted one (PurRDF mints no vocabulary IRIs of its own; see
[AGENTS.md](https://github.com/Blackcat-Informatics/purrdf/blob/main/AGENTS.md)'s
"NOT an ontology" contract). Supply a `ProvenanceNamespace` — a `prefix` (the
bare top-level JSON member key, and the XML namespace prefix) plus the XML
namespace `iri` — as `serialize`/`to_json`/`to_xml`'s fourth argument:

```rust,ignore
use purrdf_sparql_results::{ProvenanceNamespace, ResultProvenance, SparqlResultsFormat, serialize};

let namespace = ProvenanceNamespace {
    prefix: "prov".to_string(),
    iri: "https://example.org/provenance#".to_string(),
};
let provenance = ResultProvenance::default(); // or a populated one
let outcome = serialize(&result, SparqlResultsFormat::Json, &provenance, Some(&namespace))?;
```

With `namespace: None`, JSON/XML emit no provenance element/member at all,
however populated a `ResultProvenance` is — the same drop-and-flag contract
CSV/TSV always used. Where the format has no extension point at all (CSV/TSV),
the provenance is dropped loudly regardless of `namespace`, per the loss
discipline described in [Slices, Mappings & Provenance](../slices.md).

## One term-syntax authority

The crate depends only on `purrdf-core` and stays wasm-clean; term and
N-Triples syntax come exclusively from the kernel's emit primitives, so there
is exactly one term-syntax authority in the workspace — results, codecs, and
diagnostics can never disagree about how a term is written.

## Related

- [SPARQL: Querying](querying.md) — producing the `SparqlResult` in the first
  place.
- [docs.rs/purrdf-sparql-results](https://docs.rs/purrdf-sparql-results) — the
  full API reference.
