<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# SPARQL: Result Formats

[`purrdf-sparql-results`](https://docs.rs/purrdf-sparql-results) is the results
boundary of the SPARQL stack: the canonical authority for turning a
`SparqlResult` (SELECT solutions, ASK boolean, or CONSTRUCT graph) into the
four W3C SPARQL Results formats — JSON (SRJ), XML, CSV, and TSV — plus an
additive, provenance-carrying PurRDF extension where the format can carry one.
JSON and XML documents can also be read back (`from_json`, `from_xml`).

```rust,ignore
use purrdf::sparql::{serialize, ResultProvenance, SparqlResultsFormat};

// `result` is the SparqlResult produced by purrdf-sparql-eval (or any engine
// implementing the purrdf-core SparqlEngine seam).
let outcome = serialize(&result, SparqlResultsFormat::Json, &ResultProvenance::default())
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
  `SerializeOutcome::provenance_dropped` is set; the drop is never silent.

| Format | SELECT | ASK | CONSTRUCT | Provenance extension |
| --- | --- | --- | --- | --- |
| JSON (SRJ) | yes | yes | yes | yes |
| XML | yes | yes | rejected | yes |
| CSV | yes | rejected | rejected | dropped, flagged |
| TSV | yes | rejected | rejected | dropped, flagged |

## The provenance extension

The PurRDF extension is **additive**: a standard SPARQL results consumer can
read the JSON/XML documents unchanged, while a PurRDF-aware consumer can
recover per-result provenance carried alongside the bindings. Where the format
has no extension point (CSV/TSV), the provenance is dropped loudly, per the
loss discipline described in
[Slices, Mappings & Provenance](../slices.md).

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
