<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# purrdf-core

`purrdf-core` is the oxigraph-free RDF 1.2 kernel for the PURRDF Rust
workspace. It owns the shared RDF model, RDF diagnostics, the interned IR
(`TermId`/`RdfDataset`), store traits, `DatasetView`, and the oxigraph-free GTS
readers.

This crate is the **strong ring-fence** of the purrdf plan (#885, P2b): it has
**no oxigraph dependency at all**, so an accidental `use oxigraph` here is a
compile error rather than a lint miss. The oxigraph adapter lives in the sibling
`purrdf` crate, which depends on and re-exports this kernel. The invariant is
enforced structurally — `make rdf-core-hygiene` asserts no `oxigraph` package
appears in this crate's normal dependency tree.

Like `purrdf`, this crate deliberately does not emit SARIF. It exposes
structured RDF diagnostics and source locations so callers can translate them
into `purrdf-diagnostics` findings or SARIF without coupling the RDF core to a
reporting format.
