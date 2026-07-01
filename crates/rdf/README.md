<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# purrdf

`purrdf` is the first-party RDF 1.2 adapter layer over the PURRDF kernel. It
depends on and re-exports the oxigraph-free [`purrdf-core`](../rdf-core) crate
(the shared RDF model, diagnostics, interned IR, store traits, `DatasetView`, and
GTS readers) and adds parsing/materialization, Turtle normalization, statement
codecs, and GTS adapters. Python bindings live in `bindings/python` and are not
compiled into this crate.

The core/adapter split (#885, P2b) makes the oxigraph boundary a **crate
boundary**: `purrdf-core` never names oxigraph, so leaks are compile errors.
Consumers depend on `purrdf` and reach the kernel transparently through its
re-exports.

The crate deliberately does not emit SARIF. It exposes structured RDF diagnostics
and source locations so callers can translate them into `purrdf-diagnostics`
findings or SARIF without coupling the RDF core to a reporting format.
