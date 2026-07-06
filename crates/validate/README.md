<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-validate` — SARIF 2.1.0 Reporting Boundary

[![crates.io](https://img.shields.io/crates/v/purrdf-validate.svg)](https://crates.io/crates/purrdf-validate)
[![docs.rs](https://docs.rs/purrdf-validate/badge.svg)](https://docs.rs/purrdf-validate)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-validate` is the **SARIF 2.1.0 reporting boundary** of the PurRDF
toolkit. The PurRDF kernel stays *structured but SARIF-free*: parse failures are
`RdfDiagnostic`s and SHACL results are `ValidationReport`s, and neither knows
anything about SARIF or serde. This crate is where that structured data crosses
into a **source-traced, byte-deterministic SARIF 2.1.0 log** for editors, CI,
and code-scanning dashboards.

What lives here, and why here:

- A hand-rolled SARIF serde model — no heavyweight SARIF dependency.
- The mappings from PurRDF severities, rules, and source locations to SARIF
  `level` / `ruleId` / `physicalLocation` / `logicalLocation`.
- The resolution of runtime-only provenance ids to public IRIs at the
  serialization boundary — numeric ids never enter the emitted JSON.

Hosting the writer in this leaf keeps the kernel ring-fence intact:
`purrdf-core` and `purrdf-shapes` never gain a SARIF or serde-derive concern.
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

let sarif = validate_to_sarif_string(shapes, data, &SarifOptions::default())
    .expect("sarif produced");
assert!(sarif.contains("\"version\": \"2.1.0\""));
```

Lower-level entry points build a `SarifLog` value instead of a string —
`build_report_sarif` for an existing SHACL `ValidationReport`, and
`build_diagnostics_sarif` for a slice of parser/codec `RdfDiagnostic`s — so a
host can merge runs or post-process before serializing with `to_json_pretty`.
Output is byte-deterministic: the same inputs always produce the same JSON.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
as `purrdf::validate`; depend on `purrdf-validate` directly only when you want
the reporting boundary alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
