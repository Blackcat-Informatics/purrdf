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
they belong to the RDF **syntax-codec** conformance suite, not to this zero-dep
IRI kernel.

They are vendored there, in `crates/rdf/tests/corpus/w3c/turtle/iri/` and
`crates/rdf/tests/corpus/w3c/trig/iri/`, and are run by
`crates/rdf/tests/native_codec_conformance.rs` — parsed, round-tripped, and
compared against each case's `mf:result`. Base-IRI resolution is thus exercised
end-to-end there; here it is exercised directly against
`purrdf_iri::Iri::resolve` using the RFC's own normative table. The two halves
are deliberately redundant: the kernel table pins the arithmetic, the codec
suite pins that a document's `@base`/`xml:base`/retrieval IRI actually reaches
it.

## The normative sources (verbatim)

| Test file | Source of truth | Nature |
|-----------|-----------------|--------|
| `resolution.rs` | **RFC 3986 §5.4.1** (normal examples) and **§5.4.2** (abnormal examples), base `http://a/b/c/d;p?q` | The canonical reference-resolution table every conformant URI library is measured against — transcribed verbatim. |
| `w3c_iri.rs` — `valid_uris_and_iris` | **RFC 3986 §1.1.2** worked examples + representative absolute IRIs of the shape W3C `rdf-tests` uses | Positive validity corpus. |
| `w3c_iri.rs` — `valid_iri_only` | **RFC 3987 §3.1** example IRIs (non-ASCII `ucschar`: Japanese, Devanagari, Cyrillic, Latin-1) | IRI-valid but URI-invalid corpus. |
| `w3c_iri.rs` — `invalid_iris_are_rejected` | **RFC 3987 §2.2 / RFC 3986 §2–§3** grammar (disallowed characters, truncated `pct-encoded`, unterminated IP-literal, non-digit port) plus the disallowed-character cases the `rdf-tests` negative Turtle IRIREF fixtures assert | Negative corpus. |
| `authority.rs` | [RFC 3986 §3.2.2](https://www.rfc-editor.org/rfc/rfc3986.html#section-3.2.2) IPv6address / IPvFuture and [§3.2.3](https://www.rfc-editor.org/rfc/rfc3986.html#section-3.2.3) generic port syntax; RFC 3987 §2.2 imports both productions | Exact grammar boundaries and lexical preservation through public IRI/base APIs. |
| `iri_suite.rs` | CURIE / prefixed-name expansion + `rdf-tests`-style IRIREF handling | First-party edge cases layered on the RFC grammar. |
| `proptest.rs` | Property-based round-trip / idempotence invariants over the RFC 3986/3987 grammar | Generative, not a fixed corpus. |
| `langtag_corpus.rs` | **RFC 5646 Appendix A** worked examples (well-formed, and the invalid set split along the §2.2.9 well-formed/valid line), the closed **§2.2.8** grandfathered list, and boundary vectors derived from the **§2.1** ABNF | `Language-Tag` well-formedness corpus; every refusal is paired with an accepted neighbor. Every vector here is either a string the RFC itself prints (Appendix A, the §2.2.8 list) or one derived by naming a §2.1 production and stepping one character or one repetition across its bound; no vector was taken from, checked against, or suggested by any implementation's test corpus. |
| `langtag_differential.rs` + `langtag_differential_vectors.txt` | **Inputs**: generated independently by a systematic sweep over the **RFC 5646 §2.1** ABNF (each of the seven `langtag` sections swept across its admissible shapes, its length/character boundaries and impostors just outside them — as a reduced full cartesian product, as one axis at full breadth in three contexts, and as every adjacent axis pair), plus the closed **§2.2.8** grandfathered list and the **Appendix A** worked examples with case variants, plus structural inputs (empty, hyphen placement, over-length subtags, non-ASCII, C0 controls). **Verdicts**: labelled once by `oxilangtag` 0.1.6 (`LanguageTag::parse(..).is_ok()`) run as a one-time external oracle over those inputs. | Frozen differential acceptance table (3935 vectors) that makes the "same accepted language as the replaced dependency" claim **falsifiable**. See the fidelity note below on what was and was not taken from upstream. |

## Fidelity statement

The `resolution.rs` table is a **verbatim** transcription of the RFC 3986 §5.4
normative examples (the base, each reference, and each expected target are the
RFC's own strings). The `w3c_iri.rs` positive/negative strings are the RFCs' own
example IRIs plus the character classes their grammars mandate; they are faithful
to the normative text rather than fetched from a git suite, because (a) the crate
is zero-dependency and (b) no standalone W3C IRI manifest exists to vendor. Any
future divergence from these normative tables is a real bug, not a skip.

## The language-tag differential table: oracle, not source

`langtag_differential_vectors.txt` is the one fixture here whose *verdict*
column was produced by running third-party software, so its boundaries are
stated exactly:

* **What was used.** `oxilangtag` 0.1.6 was built once, outside this repository,
  in a throwaway crate, and asked `LanguageTag::parse(input).is_ok()` for each
  independently generated input. Only that boolean was kept.
* **What was not used.** No upstream source code was copied, adapted or
  consulted for the parser, and **no upstream test data was read or copied**:
  the inputs come from the RFC's own ABNF, its closed grandfathered list and its
  Appendix A, all of which are normative specification text. A verdict table is
  a measurement of observable behaviour, not an expression of the program that
  produced it.
* **What it is not.** `oxilangtag` is not a dependency of this workspace and is
  banned from re-entering it on any edge (`scripts/check-banned-deps.py`). The
  oracle run is not repeatable inside the repository by design; the table is
  frozen instead, and carries a SHA-256 of its own body that the test recomputes
  so that a later edit to a verdict cannot pass silently.

A disagreement between that table and `purrdf_iri::langtag` is a **parser**
defect, and is fixed in the parser.

Generic port syntax is `*DIGIT`: it imposes no integer-width or transport range
limit. The previous negative vector `http://h:99999/` was an incorrect
first-party restriction, not an RFC requirement. It is now a positive boundary;
non-digit ports remain negative. IP-literal vectors likewise test the complete
IPv6address or IPvFuture production, rather than a permissive character bag.
