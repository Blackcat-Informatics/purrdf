<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Provenance of the `purrdf-iri` conformance vectors

`purrdf-iri` is a **zero-dependency** crate (enforced by `make rdf-core-hygiene`).
It therefore cannot pull a test harness, parse Turtle/JSON manifests, or fetch a
suite at test time — every conformance vector is **committed inline** and
deterministic. This document is the single auditable record of where those
vectors come from, so the "verbatim / faithful to the normative source" claim is
checkable rather than asserted.

## Why there is no vendored W3C IRI *manifest*

There is **no standalone W3C IRI test suite** to vendor as a manifest tree. The
IRI-bearing fixtures in W3C `rdf-tests` (the `IRI-resolution-01/02/07/08`,
`IRIREF_datatype`, `IRI_with_*_numeric_escape`, … cases) are **RDF-syntax**
documents: they exercise base-IRI resolution *while parsing Turtle/TriG/N-Triples*
and assert on the resulting graph. Consuming them requires a full RDF parser, so
they belong to the RDF **syntax-codec** conformance suite
(`crates/rdf/tests/corpus/w3c/`), not to this zero-dep IRI kernel. Base-IRI
resolution is thus exercised end-to-end there; here it is exercised directly
against `purrdf_iri::Iri::resolve` using the RFC's own normative table.

## The normative sources (verbatim)

| Test file | Source of truth | Nature |
|-----------|-----------------|--------|
| `resolution.rs` | **RFC 3986 §5.4.1** (normal examples) and **§5.4.2** (abnormal examples), base `http://a/b/c/d;p?q` | The canonical reference-resolution table every conformant URI library is measured against — transcribed verbatim. |
| `w3c_iri.rs` — `valid_uris_and_iris` | **RFC 3986 §1.1.2** worked examples + representative absolute IRIs of the shape W3C `rdf-tests` uses | Positive validity corpus. |
| `w3c_iri.rs` — `valid_iri_only` | **RFC 3987 §3.1** example IRIs (non-ASCII `ucschar`: Japanese, Devanagari, Cyrillic, Latin-1) | IRI-valid but URI-invalid corpus. |
| `w3c_iri.rs` — `invalid_iris_are_rejected` | **RFC 3987 §2.2 / RFC 3986 §2–§3** grammar (disallowed characters, truncated `pct-encoded`, unterminated IP-literal, out-of-range port) plus the disallowed-character cases the `rdf-tests` negative Turtle IRIREF fixtures assert | Negative corpus. |
| `iri_suite.rs` | CURIE / prefixed-name expansion + `rdf-tests`-style IRIREF handling | First-party edge cases layered on the RFC grammar. |
| `proptest.rs` | Property-based round-trip / idempotence invariants over the RFC 3986/3987 grammar | Generative, not a fixed corpus. |

## Fidelity statement

The `resolution.rs` table is a **verbatim** transcription of the RFC 3986 §5.4
normative examples (the base, each reference, and each expected target are the
RFC's own strings). The `w3c_iri.rs` positive/negative strings are the RFCs' own
example IRIs plus the character classes their grammars mandate; they are faithful
to the normative text rather than fetched from a git suite, because (a) the crate
is zero-dependency and (b) no standalone W3C IRI manifest exists to vendor. Any
future divergence from these normative tables is a real bug, not a skip.
