<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# First-party `CONSTRUCT` conformance corpus

First-party, not vendored. Run by
`crates/sparql-conformance/tests/construct_corpus.rs`, which reports the
`SPARQL CONSTRUCT (first-party corpus)` row of the conformance matrix.

## Why it exists

The vendored W3C `sparql11/construct` group grades the triple-producing form
only, and it grades it as part of a four-digit aggregate row. A consumer asking
"is `CONSTRUCT` covered, and is the quad-producing `CONSTRUCT GRAPH <iri>` form
covered" needs a row it can read, with goldens it can inspect. That is this
corpus.

## Layout

| Path | Role |
|---|---|
| `manifest.ttl` | the case list, in the W3C `mf:` manifest shape |
| `data.ttl` | the default-graph fixture (`qt:data`) |
| `worlds.ttl` | a named-graph fixture (`qt:graphData`) |
| `<case>.rq` | the query |
| `<case>.ttl` | a **triple**-form expectation — a graph, so every statement is in the default graph |
| `<case>.nq` | a **quad**-form expectation — N-Quads, carrying the target graph on every line |

## The pairing, and why the file extensions carry meaning

Every rule that governs template instantiation — the unbound-variable skip, the
ill-formed-triple skip, per-row blank-node freshness — is exercised under BOTH
forms, because `CONSTRUCT GRAPH` is defined to be the ordinary instantiation
with a graph name attached, and a corpus that only tested the new form could
not say that.

The expectations are compared as canonical N-Quads, so the choice of expectation
file is itself a check: a Turtle expectation parses into the default graph, so a
triple-form case whose output started carrying a graph term would stop matching,
and an N-Quads expectation names the target graph on every line, so a quad-form
case whose output fell back to the default graph would stop matching too.

## Freezing

Byte-frozen: `scripts/check-corpus-frozen.py` recomputes a SHA-256 over every
payload file here on `make check` and compares it to
`scripts/conformance-frozen/sparql-conformance-corpus-construct.sha256`. This
`README.md` is deliberately not covered, so editing prose never requires
regenerating the digest.
