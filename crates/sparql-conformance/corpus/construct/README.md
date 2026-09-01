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
| `annotated.ttl` | an RDF 1.2 fixture whose reification layer exposes a triple term |
| `worlds.ttl` | a named-graph fixture (`qt:graphData`) |
| `<case>.rq` | the query |
| `<case>.ttl` | a **triple**-form expectation — a graph, so every statement is in the default graph |
| `<case>.nq` | a **quad**-form expectation — N-Quads, carrying the target graph on every line |

## The pairing, and why the file extensions carry meaning

Every triple-form evaluation case has a `graph`-prefixed quad-form counterpart —
the rewrite, the `CONSTRUCT WHERE` short form, the unbound-variable skip, the
literal-subject skip, the triple-term-subject skip, per-row blank-node freshness,
a `GRAPH`-scoped `WHERE`, the solution modifiers, and the RDF 1.2 reifier +
annotation template. That is
checked by `construct_corpus.rs`, not merely asserted here, because
`CONSTRUCT GRAPH` is defined to be the ordinary instantiation with a graph name
attached and a corpus that only tested the new form could not say that.

Some quad-form cases have no triple-form twin, by nature rather than by
omission: the syntax verdicts grade the quad grammar itself, and
`graphReifierScope` needs TWO graphs to say anything at all.

## The §16.2 subject rule, on every term and every depth it refuses

A subject position — asserted, or inside a triple term — is an IRI or a blank
node. Two kinds of term are therefore ill-formed there, at either of the two
depths a template can reach, and an instantiation that produces any of them is
SKIPPED rather than errored:

| Case (and its `graph`-prefixed twin) | The term, and where it lands |
|---|---|
| `illFormedSkipped` | a literal, asserted subject |
| `tripleTermSubjectSkipped` | an RDF 1.2 triple term, asserted subject |
| `nestedTripleTermSubjectSkipped` | an RDF 1.2 triple term, subject of a triple term nested in the object |

None of these is a curiosity. RDF 1.2 data puts a triple term within reach of
an ordinary triple pattern — `?r rdf:reifies ?t` binds one — so a template that
carries that binding into a subject position is something an engine meets on
real input, and a corpus that graded only the literal half would call an engine
conformant while it hard-failed on annotated data. `annotated.ttl` is that
input, and every case keeps one WELL-FORMED triple beside the skipped one so
the expectation distinguishes "the skip fired" from "the `WHERE` matched
nothing".

The nested case is graded separately from the asserted one because a different
mechanism decides it — the asserted level is one predicate, the nested level is
that predicate's RECURSION — and because the two fail differently. An
ill-formed ASSERTED subject is refused downstream whatever the template rule
says. An ill-formed NESTED one is not: an engine that enforced the term model
only at the asserted level would emit the statement at exit status zero, into a
document its own N-Triples/N-Quads/Turtle readers then refuse to parse, and on
an `UPDATE` path would persist it. A green corpus that could not see that is
the failure this case exists to make impossible.

## The grammar boundary, from both sides

The accepted quad-template grammar is

```text
ConstructQuads           ::= TriplesTemplate? ( ConstructQuadsNotTriples '.'? TriplesTemplate? )*
ConstructQuadsNotTriples ::= ( 'GRAPH' VarOrIri )? '{' TriplesTemplate? '}'
```

plus the whole-template `CONSTRUCT GRAPH VarOrIri …` shorthand. `graphSyntax`
pins one spelling that parses. A corpus that stopped there would let the grammar
widen without anyone noticing, so each spelling that is genuinely REFUSED is
pinned too, one case per rule:

| Case | The rule it pins |
|---|---|
| `graphNoNameSyntax` | the shorthand's graph name is not optional |
| `graphLiteralNameSyntax` | the name is a `VarOrIri` — a literal is not one |
| `graphBlankNameSyntax` | …and neither is a blank node |
| `graphNestedBlockSyntax` | a block body is a `TriplesTemplate`; blocks do not nest |
| `graphBlockNoBracesSyntax` | a block's braces are not optional |
| `graphShortFormBlockSyntax` | the short form's one block is read twice, as template AND as `WHERE` algebra, so it admits no `GRAPH` block |
| `graphBlockPathSyntax` | a template asserts triples, so the property-path ban holds inside a `GRAPH` block exactly as it does outside one |

These are empirical: each was confirmed against the parser before it was
declared, so the corpus measures the boundary rather than assuming it.

## The RDF 1.2 statement layer

Reifier declarations and annotations are a separate emission path — they do not
travel through `push_quad` — so a regression that left them in the default graph
beside the target graph's quads would be invisible to a corpus of plain triples.
`reified` and `graphReified` are the same reifier + annotation template under
both forms, and their expectations differ exactly by the graph term.

`graphReifierScope` pins the layer's **per-graph keying**: one reifier id, one
annotation predicate, two graphs. In the graph that declares the reifier the
statement is an annotation; in the graph that does not, the identical shape is an
ordinary quad. The two canonicalize differently, so the case fails the moment
either the template evaluator or the N-Quads reader degrades to "a reifier
declared anywhere in the output".

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
