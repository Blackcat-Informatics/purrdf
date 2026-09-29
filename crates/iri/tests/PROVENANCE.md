<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

# Provenance of the `purrdf-iri` conformance vectors

`purrdf-iri` takes **no third-party dependency** — its one runtime dependency is
the zero-dependency `purrdf-lex` (enforced by `make rdf-core-hygiene`).
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
| `property.rs` | Property-based round-trip / idempotence invariants over the RFC 3986/3987 grammar | Generative, not a fixed corpus. |
| `langtag_corpus.rs` | **RFC 5646 Appendix A** worked examples (well-formed, and the invalid set split along the §2.2.9 well-formed/valid line), the closed **§2.2.8** grandfathered list, and boundary vectors derived from the **§2.1** ABNF | `Language-Tag` well-formedness corpus; every refusal is paired with an accepted neighbor. Every vector here is either a string the RFC itself prints (Appendix A, the §2.2.8 list) or one derived by naming a §2.1 production and stepping one character or one repetition across its bound; no vector is taken from, checked against, or suggested by any implementation's test corpus. See the clean-room note below for the three that once were. |
| `langtag_differential.rs` + `langtag_differential_vectors.txt` | **Inputs**: generated independently by a systematic sweep over the **RFC 5646 §2.1** ABNF (each of the seven `langtag` sections swept across its admissible shapes, its length/character boundaries and impostors just outside them — as a reduced full cartesian product, as one axis at full breadth in three contexts, and as every adjacent axis pair), plus the closed **§2.2.8** grandfathered list and the **Appendix A** worked examples with case variants, plus structural inputs (empty, hyphen placement, over-length subtags, non-ASCII, C0 controls). **Verdicts**: labelled once by an independent external RFC 5646 implementation, run outside this repository as a one-time oracle over those inputs. | Frozen differential acceptance table (3935 vectors) that makes the "same accepted language as the replaced dependency" claim **falsifiable**. See the fidelity note below on what was and was not taken from upstream. |
| `host_differential.rs` + `host_differential_vectors.txt` | **Inputs**: 20,000 distinct address-shaped strings drawn from the testkit choice stream under a fixed seed (dotted quads whose octets straddle 255 and carry leading zeros; colon-separated hex groups, compressed and not, with dotted tails and zones; point mutations). **Verdicts**: the `ipv4` and `ipv6` format checks `purrdf-jsonschema` carried before it called `purrdf_iri::host`, recorded once while they existed. | Frozen differential acceptance table for **RFC 3986 §3.2.2** `IPv4address` and `IPv6address` (20,000 vectors, two verdicts each). See the note below. |
| `idna.rs` — IdnaTestV2 lanes | **`IdnaTestV2.txt` 17.0.0** (Unicode, Inc.), vendored verbatim at `crates/iri/unicode/17.0.0/`; its header's FORMAT section defines the columns | Two lanes (`to_ascii`, `to_ascii_mapped`) through one committed row filter whose every excluded class cites an RFC 5891/5892 clause; zero failures and the included counts are asserted. See the note below. |
| `idna.rs` — Punycode | **RFC 3492 §7.1** sample strings (A)–(S), code points and Punycode as printed | Both directions; the RFC's mixed-case annotation is ignored where the encoder is compared. |
| `idna.rs` — `to_uri` | **RFC 3987 §3.1** examples (the `ireg-name` ToASCII variant, the `%09` path, the supplementary-plane path) | Each result re-parses under `parse_uri` to the same spans. |
| `idna.rs` — refusals | Boundaries derived from the RFC 5890 §2.3.1 lengths, RFC 5891 §4.2 label rules, RFC 5892 Appendix A and RFC 5893 §2 | Every refusal is paired with an accepted neighbour. |

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

* **What was used.** An independent external RFC 5646 implementation was built
  once, outside this repository, in a throwaway crate, and asked whether each
  independently generated input is well-formed. Only that boolean was kept.
* **What was not used.** No upstream source code was copied, adapted or
  consulted for the parser, and **no upstream test data was read or copied**:
  the inputs come from the RFC's own ABNF, its closed grandfathered list and its
  Appendix A, all of which are normative specification text. A verdict table is
  a measurement of observable behaviour, not an expression of the program that
  produced it.
* **What it is not.** The oracle implementation is not a dependency of this
  workspace. The oracle run is not repeatable inside the repository by design; the table is
  frozen instead, and carries a SHA-256 of its own body that the test recomputes
  so that a later edit to a verdict cannot pass silently.

A disagreement between that table and `purrdf_iri::langtag` is a **parser**
defect, and is fixed in the parser.

## The clean-room note on `langtag_corpus.rs`

The row above claims that no vector in `langtag_corpus.rs` came from an
implementation's test corpus. That claim was **false when first written**: an
audit found three strings there — a `4ALPHA`-language-plus-`script` vector, its
`4ALPHA`-language-plus-3ALPHA refusal, and a maximum-length `5*8ALPHA` language
carrying a `privateuse` section — byte-identical to vectors in the test suite of
the crate that was replaced, one of them carried across with its rationale
comment. None appears anywhere in RFC 5646, so none could have been derived the
way the row describes.

All three were replaced with vectors constructed by the stated method: name the
§2.1 production, name the bound, and step one character across it. The rules
they pin are unchanged — the `4ALPHA` reserved-language branch, the `5*8ALPHA`
registered-language branch, that `["-" extlang]` hangs off `2*3ALPHA` only, and
the accepted/refused neighbour pairing — because the rules were never the
problem; the strings were. The row is written in the present tense for that
reason: it describes what the file contains, and this note records that it has
not always been true of it.

Generic port syntax is `*DIGIT`: it imposes no integer-width or transport range
limit. The previous negative vector `http://h:99999/` was an incorrect
first-party restriction, not an RFC requirement. It is now a positive boundary;
non-digit ports remain negative. IP-literal vectors likewise test the complete
IPv6address or IPvFuture production, rather than a permissive character bag.

## The host differential table

`host_differential_vectors.txt` freezes verdicts for RFC 3986 §3.2.2
`IPv4address` (equivalently, RFC 2673 §3.2's dotted quad) and `IPv6address`
(equivalently, RFC 4291 §2.2's text form) membership — the same productions
`purrdf-jsonschema`'s `ipv4` and `ipv6` format checks decide by calling
`purrdf_iri::host::is_ipv4_address` and `purrdf_iri::host::is_ipv6_address`.

* **What was taken.** Their `accept`/`reject` answers, and nothing else. They
  were first-party code, and none of it was carried into `host`: the
  predicates there are the RFC 3986 `dec-octet` production and the standard
  library's address parser.
* **Disagreements.** None. Replayed against `purrdf_iri::host`, all 20,000
  records agree on both productions. The languages coincide by the RFC text:
  RFC 2673's dotted quad is RFC 3986's `IPv4address` (four decimal values
  0–255, no leading zero), and RFC 4291's text form, with the embedded IPv4
  part spelled as that `IPv4address` and no zone, is RFC 3986's nine
  `IPv6address` alternatives. The table exercises both verdicts of both
  productions at least a thousand times each, and the replay test asserts it.

The same answers decide an address in a URI: an input the table accepts as
`IPv4address` is a host `parse_uri` accepts, and one it accepts as
`IPv6address` is an IP-literal `parse_uri` accepts, while one it refuses is
refused inside brackets unless it begins with the `v` of an `IPvFuture`.

## The IdnaTestV2 row filter

`IdnaTestV2.txt` is the UTS 46 test file, and UTS 46 is not IDNA2008: it
maps input with its own table, accepts some characters IDNA2008 disallows,
and does not test CONTEXTO rules. So the file is an oracle for this crate only
through the filter `Row::verdict` in `idna.rs`, which is one predicate over
the source, toUnicode, toAsciiN and status columns. With 6391 data rows:

* **Unmapped lane** (`to_ascii(source)`): 2960 rows included. Excluded:
  2 ill-formed (an unpaired surrogate; RFC 5891 §4.1/§5.2 input is Unicode),
  3363 mapped by UTS 46 (RFC 5891 §5.2: mapping is the application's),
  14 holding a CONTEXTO code point where UTS 46 accepts (RFC 5891 §4.2.3.3,
  RFC 5892 Appendix A), 52 holding a code point outside RFC 5892 §2.1
  LetterDigits that no earlier rule admits where UTS 46 accepts (RFC 5892 §3).
* **Mapped lane** (`to_ascii_mapped(source)`): 6202 rows included. Excluded:
  2 ill-formed, 32 CONTEXTO, 140 outside LetterDigits, and two named mapping
  differences (RFC 5891 §5.2): 2 rows holding U+1E9E, which NFKC_Casefold folds
  to `ss`, and 13 rows whose only UTS 46 objection is a tag character that
  NFKC_Casefold erases.

The rows excluded as outside LetterDigits are still checked: both functions
must refuse them. The General_Category that decides that class is read from
the vendored `UnicodeData.txt` by the test itself, independently of the
generated tables under test.
