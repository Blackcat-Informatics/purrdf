<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-validate` — SARIF 2.1.0 Reporting Boundary

[![crates.io](https://img.shields.io/crates/v/purrdf-validate.svg)](https://crates.io/crates/purrdf-validate)
[![docs.rs](https://docs.rs/purrdf-validate/badge.svg)](https://docs.rs/purrdf-validate)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-validate` is the **SARIF 2.1.0 reporting boundary** of the PurRDF
toolkit. The PurRDF kernel stays *structured but SARIF-free*: parse failures are
`RdfDiagnostic`s and SHACL results are `ValidationReport`s, and neither knows
anything about SARIF. This crate is where that structured data crosses
into a **source-traced, byte-deterministic SARIF 2.1.0 log** for editors, CI,
and code-scanning dashboards.

What lives here, and why here:

- A hand-rolled SARIF object model written through `purrdf_lex::json` — no
  heavyweight SARIF dependency.
- The mappings from PurRDF severities, rules, and source locations to SARIF
  `level` / `ruleId` / `physicalLocation` / `logicalLocation`.
- The resolution of runtime-only provenance ids to public IRIs at the
  serialization boundary — numeric ids never enter the emitted JSON.

Hosting the writer in this leaf keeps the kernel ring-fence intact:
`purrdf-core` and `purrdf-shapes` never gain a SARIF concern.
Like every PurRDF release crate, it is pure library code with no ambient I/O
and builds cleanly for `wasm32-unknown-unknown`.

## Usage

```sh
cargo add purrdf-validate
```

Validate a SHACL shapes + data pair straight to a SARIF string:

```rust
use purrdf_validate::{validate_to_sarif_string, SarifOptions};

let shapes = r#"
    @prefix sh:  <http://www.w3.org/ns/shacl#> .
    @prefix ex:  <http://example.org/> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    ex:PersonShape a sh:NodeShape ;
      sh:targetClass ex:Person ;
      sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .
"#;
let data = r#"<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .
<http://example.org/alice> <http://example.org/age> "nope" .
"#;

// No shapes base, default options, and an empty `owl:imports` table.
let sarif = validate_to_sarif_string(shapes, None, data, &SarifOptions::default(), &[])
    .expect("sarif produced");
assert!(sarif.contains("\"version\": \"2.1.0\""));
```

Every shapes-graph entry point here takes the shapes graph's `owl:imports` table:
`(ontology IRI, Turtle document)` pairs, each document parsed under its IRI. A shapes
graph that imports a document the table does not supply — and does not already
declare (`<X> a owl:Ontology`, or an ontology whose `owl:versionIRI` is `<X>`) — is
refused with the typed `ShapesError::Imports`, the same refusal the Python,
WebAssembly and C hosts carry, rather than validated without it. PurRDF fetches
nothing.

The same table resolves the data graph's `sh:shapesGraph` links (SHACL 1.2 Core §6.4):
a `sh:shapesGraph` on a `sh:DataGraph` node of the data graph names a graph that is
looked up, followed through its own `owl:imports` and unioned into the shapes graph —
the shapes document may even be empty — or refused by name
(`unresolved-shapes-graph-link`). On any other node it is data. A prepared product
cannot take a graph in, so validating with one refuses a link it does not hold
(`unheld-shapes-graph-link`). A closure that holds two versions of one series, or a
graph another declares `owl:incompatibleWith`, is ill-formed (SHACL 1.2 Core §1.3,
§6.1) and refused as `incompatible-import-versions`, naming both graphs.

`SarifOptions::validation` carries the request's `ValidationOptions`: the
conformance-disallow set, and SHACL 1.2 Core §6.3's `subClassOfInShapesGraph`
(`with_subclass_of_in_shapes_graph(true)`), which reads the shapes graph's
`rdfs:subClassOf` triples, in addition to the data graph's, wherever SHACL type
decides class membership. It is off by default, the specification's default.

Lower-level entry points build a `SarifLog` value instead of a string —
`build_report_sarif` for an existing SHACL `ValidationReport`, and
`build_diagnostics_sarif` for a slice of parser/codec `RdfDiagnostic`s — so a
host can merge runs or post-process before serializing with `to_json_pretty`.
Output is byte-deterministic: the same inputs always produce the same JSON.

## Complete contextual reports

The additive Rust doors `validate_complete_documents`, `validate_complete_sources`
and `validate_complete_product` return `CompleteValidationReport` with the
request's `ValidationOptions`. Select a named SHACL bundle with `with_profile`;
its required native XPath law follows that selection. `complete_profile` decodes
only supported exact identifiers and reports an unsupported identifier without
falling back to a default.

`complete_validation_status` distinguishes conformance, nonconformance, typed
admission/source/semantic/resource/product refusals and other execution errors.
A refusal never yields a partial conformance verdict. Native query resource
classification reads the stable `Resource::code` identity, never diagnostic
wording.

`complete_report_payload` and `complete_report_to_json` carry the full N-Quads
report, its dedicated root, retained source graphs and authored-blank
correspondence. Acquisition-local indices distinguish data from independently
acquired shapes even when labels agree; deliberately shared acquisitions are
encoded once. Each correspondence row retains the emitted report label and the
original source scope and label. Generated report/path/query blanks are not
presented as authored nodes.

`complete_report_to_sarif_string` uses the existing SARIF mapper and attaches the
complete payload as `runs[0].properties.shaclCompleteReport`. Required constraint
identity, recursive details and multiplicity remain together in that RDF graph;
they are not inferred from sorted SARIF result positions. Existing host exports,
default payloads and SARIF bodies keep their compatibility projection.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C (the GTS container itself reaches Python and C, not the wasm package). Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
as `purrdf::validate`; depend on `purrdf-validate` directly only when you want
the reporting boundary alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.98, stable toolchain only).

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)
