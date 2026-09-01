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
| `data.ttl` | the default-graph fixture for clauses 1-3 (`qt:data`) |
| `statements.ttl` | the RDF 1.2 statement-layer fixture for clause 4 (`qt:data`) |
| `statements-graphs.trig` | clause 4 crossed with graph scope, every row in a named graph (`qt:data`) |
| `worlds.ttl` | a named-graph fixture (`qt:graphData`) |
| `<case>.rq` | the query |
| `<case>.ttl` / `<case>.nq` | the expected description |

The fixtures are built so each clause of the SCBD definition can be isolated.

`data.ttl` carries clauses 1-3: an outgoing edge from the described subject
(clause 1), an incoming edge to it (clause 2), a blank-node hop off it
(clause 3), and a literal object (which is not a describable subject).

`statements.ttl` carries clause 4 — the RDF 1.2 statement layer, i.e. the
reifiers whose reified triple's subject **or** object lies in the closure,
together with their annotations. Both sides of that disjunction are measured
separately, and the object side is measured on a term that is the subject of
nothing and whose reified triple is unasserted, so the whole of its description
had to arrive through the object half. It also carries a reifier about a triple
no case reaches, so "the reifiers about the closure" is graded against something
narrower than "every reifier in the dataset", and a blank reifier with a blank
annotation object, so clauses 3 and 4 are exercised composed.

`statements-graphs.trig` carries clause 4 crossed with **graph scope**, and it is
TriG for a reason: every case above reads a default-graph-only fixture, so its
expectation is graph-less on every row and cannot observe where a described
statement *lands* — only that it was selected. These cases expect N-Quads, so the
graph of each reifier declaration and each annotation is pinned. They fix the
selection/emission split the extractor documents: a declaration is selected by its
reified triple's subject-or-object membership in the closure and by **nothing
else** — graph membership takes no part — and is re-emitted into the graph that
declared it. So a declaration in one graph about a triple asserted in another is
kept, and kept where it was declared; and an annotation rides with the
*declaration* it annotates, not with the reifier resource, which one reifier id
declared in two graphs (each annotated with the same predicate) measures.

## Freezing

Byte-frozen: `scripts/check-corpus-frozen.py` recomputes a SHA-256 over every
payload file here on `make check` and compares it to
`scripts/conformance-frozen/sparql-conformance-corpus-describe.sha256`. This
`README.md` is deliberately not covered.
