<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-rdf` — RDF 1.2 Implementation Layer

[![crates.io](https://img.shields.io/crates/v/purrdf-rdf.svg)](https://crates.io/crates/purrdf-rdf)
[![docs.rs](https://docs.rs/purrdf-rdf/badge.svg)](https://docs.rs/purrdf-rdf)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-rdf` is the RDF 1.2 implementation layer of the PurRDF toolkit: the
narrow waist between transport/runtime stores (such as the GTS container) and
consumers like SHACL, SPARQL, and SARIF reporting. It depends on and re-exports
the ring-fenced [`purrdf-core`](https://crates.io/crates/purrdf-core) kernel
(the interned IR, diagnostics, store traits, `DatasetView`, provenance, and the
loss ledger) and adds what the kernel deliberately leaves out:

- **Native text codecs** — first-party parsers/serializers for Turtle, TriG,
  N-Triples, N-Quads, and RDF/XML, plus JSON-LD (star) and YAML-LD; parsing
  can optionally record a source-position span table for diagnostics.
- **Native OKF codec** — bidirectional Open Knowledge Format Markdown bundles
  over the RDF event seams, with caller-owned vocabulary, deterministic YAML,
  and an always-computed loss ledger.
- **RDF 1.2 statement layer** — reifier bindings and annotations survive every
  star-capable round-trip; star-incapable projections drop them *loudly*, with
  the realized count handed to the loss ledger.
- **Canonicalization entry points** — W3C RDFC-1.0 (`canonicalize`) and
  canonical flat N-Quads over the frozen IR.
- **GTS adapters** — import/export between `RdfDataset` and the
  [`purrdf-gts`](https://crates.io/crates/purrdf-gts) container, including
  snapshot composition and content-chain verification.
- **Describe & normalization** — per-subject Symmetric-CBD extraction and a
  review-friendly Turtle normalizer.
- **RDF 1.2 visualization** — a statement-centric projection, renderer-neutral
  semantic scene, deterministic layout, statement table, and self-contained SVG
  whose embedded JSON preserves assertions, triple terms, reifiers, annotations,
  graph context, dialect diagnostics, and element-to-model identities.
- **Graph, tabular, and research-object carriers** — deterministic LPG CSV,
  Neo4j CSV, openCypher, GraphML, exact and curated CSVW, OBO Graphs 0.3.2, SKOS,
  Croissant 1.1, RO-Crate 1.3, DataCite 4.6, DCAT 3, and Frictionless Data
  Package v1 over one caller-configured, resource-bounded archive API with an
  always-computed loss ledger.

The crate is PyO3-free and oxigraph-free (like the whole workspace), keeps
reporting structured but SARIF-free — callers translate `RdfDiagnostic`s into
SARIF via [`purrdf-validate`](https://crates.io/crates/purrdf-validate) — and
builds cleanly for `wasm32-unknown-unknown`.

## Usage

```sh
cargo add purrdf-rdf
```

```rust
use purrdf_rdf::{parse_dataset, serialize_dataset, SerializeGraph};

let turtle = br#"
    @prefix ex: <https://example.org/> .
    ex:cat ex:says "meow" .
"#;

// Parse into the frozen, value-interned RDF 1.2 dataset IR.
let ds = parse_dataset(turtle, "text/turtle", None).expect("valid Turtle");
assert_eq!(ds.quad_count(), 1);

// Serialize back out through any native codec — byte-deterministic output.
let nq = serialize_dataset(&ds, "application/n-quads", SerializeGraph::Dataset)
    .expect("serializes");
assert!(String::from_utf8(nq).unwrap().contains("meow"));
```

The visualization surface is available as `purrdf_rdf::viz` (and as
`purrdf::viz` through the umbrella crate):

```rust
use purrdf_rdf::viz::{VizRenderOptions, VizSpec, render_dataset_svg};

let document = render_dataset_svg(&ds, &VizSpec::default(), &VizRenderOptions::default())
    .expect("renders");
assert!(document.svg.contains("purrdf-viz-export"));
assert_eq!(document.export.model.statements.len(), 1);
```

`project_dataset` returns the semantic model alone, `project_dataset_export`
adds the scene, geometry, hashes, diagnostics, and SVG element index, and
`render_dataset_svg` emits deterministic SVG plus the same structured export.
Compact, exact incidence, and statement-table modes all derive from one model;
quotation never implies assertion, and reifier identity remains distinct from
the structural triple term.

Malformed input is a typed `RdfDiagnostic` with a source location where the
codec can provide one — never a silent partial parse.

### Open Knowledge Format bundles

OKF uses a deterministic in-memory bundle so the same API works on native and
`wasm32-unknown-unknown` targets. PurRDF never chooses a directory, archive, or
vocabulary for the caller: filesystem materialization is outside the codec, and
the namespace, document base, and recognized frontmatter keys are mandatory.

```rust
use purrdf_rdf::{
    DatasetSink, OkfBundle, OkfConfig, lift_okf_bundle, write_okf_bundle,
};

let config = OkfConfig::new(
    "https://example.org/okf#",
    "https://example.org/doc/",
    ["type", "title"],
)?;
let bundle = OkfBundle::from_documents([(
    "concept.md",
    "---\ntype: Concept\ntitle: Example\n---\nBody.\n",
)])?;

let mut sink = DatasetSink::new();
let read = lift_okf_bundle(&bundle, &config, &mut sink)?;
assert!(read.losses.is_empty());
let dataset = sink.into_dataset().expect("the lift finished the sink");

let written = write_okf_bundle(&dataset, &config)?;
assert!(written.losses.is_empty());
assert_eq!(written.documents, 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The reader drives any `RdfEventSink`; the writer is itself an
`RdfDatasetVisitor`. Relative Markdown links preserve their exact RDF 1.2
reifier and occurrence annotations. RDF rows outside the configured OKF
profile are omitted only with deterministic, source-located `LossLedger`
entries; ambiguous profile data hard-fails.

### Graph, tabular, and research-object projection archives

All thirteen profiles use the same canonical USTAR package surface and strict,
profile-tagged configuration. PurRDF does not choose vocabulary, identity,
profile context, or resource limits for the caller.

| Profile | Project | Lift | Contract |
| --- | :---: | :---: | --- |
| `lpg-csv` | yes | yes | Generic nodes/edges CSV over the canonical LPG model |
| `neo4j-csv` | yes | yes | Neo4j Admin Import CSV over the same model |
| `open-cypher` | yes | yes | Injection-safe closed `CREATE` grammar |
| `graphml` | yes | yes | GraphML 1.0 with strict XML validation |
| `csvw-exact` | yes | yes | Lossless RDF 1.2 tables and CSVW metadata |
| `csvw-terms` | yes | no | Caller-declared scoped entity tables with located losses |
| `obo-graphs` | yes | no | OBO Graphs 0.3.2 view with located losses |
| `skos` | yes | no | SKOS Turtle view with located losses |
| `croissant-1.1` | yes | yes | Croissant 1.1 through the shared research-object model |
| `ro-crate-1.3` | yes | yes | RO-Crate 1.3 flattened JSON-LD graph |
| `datacite-4.6` | yes | yes | Namespace-aware DataCite Metadata Schema 4.6 XML |
| `dcat-3` | yes | yes | DCAT 3 over the native offline JSON-LD engine |
| `frictionless-data-package-1` | yes | yes | Frictionless Data Package v1 JSON |

The LPG carriers include exact RDF sideband for reconstruction, while their
native property-graph interpretation remains a semantic lowering and is
therefore ledgered. `csvw-exact` preserves terms, graph placement, recursive
triple terms, reifier bindings, annotations, language, direction, and datatype
with an empty ledger. Curated CSVW terms, OBO Graphs, and SKOS are structurally
write-only: `LiftProfile` has no variants for them.

`csvw-terms` is a generic caller-authored entity-table lens, not an ontology
model. Its mandatory configuration explicitly selects source graphs, row
membership by type and subject IRI, ordered predicate columns, exact IRI or
literal facets, one/many cardinality, artifact identities, and all limits. Rows
and multivalues are canonically sorted; ambiguous single values and separator
collisions fail. Every unrepresented RDF 1.2 row receives a source-located loss
entry, including named-graph placement, empty graphs, reifiers, and annotations.
Use `csvw-exact` for archival or reverse mapping and `csvw-terms` for compact,
human-facing tables.

LPG scope is mandatory. `LpgScope::all()` explicitly requests the complete
dataset; selective scope can include/exclude exact named graphs and predicates
and filter node/edge types. Independent input-record, model-record, node, edge,
artifact, body-byte, archive-byte, and term-depth limits fail before excess.
The direct `project_lpg_artifacts_to_sink` path emits transactional chunks no
larger than 16 KiB and reports structured progress. It retains the bounded,
canonically sorted selected LPG model but does not retain complete artifact
bodies or a USTAR archive; `project_archive` is the materializing convenience
path. The engine deliberately does not paginate an exactly reversible carrier.

The five research-object codecs share one typed semantic pivot. Their mandatory
configuration supplies every RDF role and identity plus each native context,
schema, controlled value, or profile identity. JSON-LD context interpretation
is offline and caller-complete; native readers reject drift and record every
unsupported construct in a located runtime ledger.

```rust
use purrdf_rdf::{
    LiftProfile, LpgConfig, LpgExecutionLimits, LpgScope, ProjectionConfig,
    ProjectionLimits, ProjectionProfile, lift_archive, parse_dataset,
    project_archive,
};

let dataset = parse_dataset(
    b"<https://example.org/alice> <https://example.org/knows> <https://example.org/bob> .",
    "text/turtle",
    None,
)?;
let limits = ProjectionLimits::new(16, 1_000_000, 4_000_000, 5_000_000, 16)?;
let config = ProjectionConfig::LpgCsv(LpgConfig::new(
    "https://example.org/type",
    LpgScope::all(),
    limits,
    LpgExecutionLimits::new(1_000, 1_000, 1_000, 1_000)?,
)?);
let projected = project_archive(dataset.as_ref(), ProjectionProfile::LpgCsv, &config)?;
let lifted = lift_archive(&projected.archive, LiftProfile::LpgCsv, &config)?;
assert_eq!(lifted.dataset.quad_count(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The runnable version is
[`examples/projection_archive.rs`](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/rdf/examples/projection_archive.rs).
All five research-object round trips are runnable in
[`examples/research_object_roundtrip.rs`](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/rdf/examples/research_object_roundtrip.rs).
The caller-configured classes/properties/individuals CSVW view is runnable in
[`examples/csvw_terms.rs`](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/rdf/examples/csvw_terms.rs).

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this entire
surface at its root and adds the SPARQL, SHACL, ShEx, and slice modules; depend
on `purrdf-rdf` directly only when you want the RDF layer alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
