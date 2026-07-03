<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# purrdf-rdf

`purrdf-rdf` is the first-party RDF 1.2 adapter layer over the PurRDF kernel. It
depends on and re-exports the oxigraph-free [`purrdf-core`](../rdf-core) crate
(the shared RDF model, diagnostics, interned IR, store traits, `DatasetView`, and
GTS readers) and adds parsing/materialization, Turtle normalization, statement
codecs, and GTS adapters. Python bindings live in `bindings/python` and are not
compiled into this crate.

The core/adapter split (P2b) makes the oxigraph boundary a **crate
boundary**: `purrdf-core` never names oxigraph, so leaks are compile errors.
Most consumers should depend on the umbrella `purrdf` crate, which re-exports
this implementation crate and includes the first-class slice and shape APIs.

The crate deliberately does not emit SARIF. It exposes structured RDF diagnostics
and source locations so callers can translate them into `purrdf-diagnostics`
findings or SARIF without coupling the RDF core to a reporting format.
