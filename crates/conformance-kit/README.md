<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# purrdf-conformance-kit

Portable conformance grading over parsed RDF and query results. Callers own
acquisition, parsing, engine execution and transport; this library compares
their observations and renders reports. It builds for native Rust and
`wasm32-unknown-unknown`.

```toml
[dev-dependencies]
purrdf-conformance-kit = "4.0.1"
```

The public modules provide:

- `inventory` and `reviews`: decode inventories and catalogs, admit reviewed
  cases, and verify exact payload and derivation identities.
- `graph` and `manifest`: read structural RDF relations, strict lists and
  `mf:` manifests from a caller-selected graph.
- `result_set` and `bag`: decode RDF result sets and compare solution bags,
  preserving multiplicity and blank-node correspondence.
- `report`: compare full SHACL reports through contextual graph isomorphism,
  with independently supplied source identities and explicit report policies.
- `outcome`: grade typed observations, retain resource and execution failures
  as distinct outcomes, and construct described, sorted EARL reports with
  scoreboard summaries.

Use `graph::Reader` with the parsed dataset and its explicit `GraphMatch`
selection. Comparison APIs accept the kernel's canonicalization control where
required. A malformed input or exhausted comparison is a `GradeError`, rather
than evidence that the implementation passed a negative semantic test.

Repository corpora and acquisition scripts are not bundled in this package;
downstream runners supply their own inputs and expected results.

## License

MIT OR Apache-2.0 OR MulanPSL-2.0. See `licenses/NOTICE.txt` and the
accompanying complete license texts.
