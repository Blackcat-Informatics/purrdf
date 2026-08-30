<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# First-party `DESCRIBE` conformance corpus

First-party, not vendored. Run by
`crates/sparql-conformance/tests/describe_corpus.rs`, which reports the
`SPARQL DESCRIBE (first-party corpus)` row of the conformance matrix.

## Why it exists

SPARQL 1.1 §16.4 leaves the description implementation-defined, so no W3C
manifest grades a `DESCRIBE` at all — the form has, until this corpus, no
conformance measurement anywhere. A first-party corpus is therefore not a
convenience here, it is the only kind of evidence the form admits: it states
what this engine returns, so a consumer can read the behaviour off a golden
instead of off a claim.

The description is the **Symmetric Concise Bounded Description**
(`purrdf_core::describe`) — the same description the CLI and the docs export
use, so `DESCRIBE` is not a fourth opinion about what "describe" means.

## Layout

| Path | Role |
|---|---|
| `manifest.ttl` | the case list, in the W3C `mf:` manifest shape |
| `data.ttl` | the default-graph fixture (`qt:data`) |
| `worlds.ttl` | a named-graph fixture (`qt:graphData`) |
| `<case>.rq` | the query |
| `<case>.ttl` / `<case>.nq` | the expected description |

`data.ttl` is built so each clause of the SCBD definition can be isolated: an
outgoing edge from the described subject, an incoming edge to it, a blank-node
hop off it, and a literal object (which is not a describable subject).

## Freezing

Byte-frozen: `scripts/check-corpus-frozen.py` recomputes a SHA-256 over every
payload file here on `make check` and compares it to
`scripts/conformance-frozen/sparql-conformance-corpus-describe.sha256`. This
`README.md` is deliberately not covered.
