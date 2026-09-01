<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf` — the PurRDF command-line interface

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf` is the native RDF 1.2 command-line tool of the PurRDF toolkit. It is a
thin, deterministic shell over the same engines the library exposes — the native
text/XML/JSON codecs, the pack container, the SPARQL 1.2 evaluator, and the
entailment closures — so anything the CLI does, it does with byte-for-byte the
same behavior as the Rust, Python, WebAssembly, and C surfaces.

Every invocation is one `Source → [transform] → Sink` pipeline, exposed as twelve
subcommands:

| Subcommand | Pipeline |
|---|---|
| [`convert`](#convert) | transcode RDF between syntaxes and the native pack container |
| [`query`](#query) | evaluate a SPARQL query over an RDF or pack data source |
| `update` | atomically apply a SPARQL UPDATE to an RDF source |
| [`reason`](#reason) | materialize an entailment regime's closure over a source graph |
| [`entails`](#entails) | decide whether a premise entails a conclusion, or answer a pattern's certain answers |
| [`consistency`](#consistency) | decide whether an OWL-Direct ontology has a model at all |
| [`validate`](#validate) | validate a data graph against a SHACL shapes graph |
| [`shex`](#shex) | validate RDF nodes against a ShEx 2.1 schema through a query shape map |
| [`describe`](#describe) | extract a resource's Symmetric Concise Bounded Description |
| [`project`](#project) | materialize a deterministic graph/tabular USTAR carrier |
| [`lift`](#lift) | reconstruct RDF from a strict bidirectional carrier |
| `pack` | verify a pack container's full canonical integrity |

A single global flag, [`--loss-ledger`](#the-loss-ledger), surfaces the
machine-readable loss record for a conversion, projection, or lift.

> **This tool mints no vocabulary.** PurRDF is a carrier, not an ontology: every
> IRI in your data is yours. The `example.org` IRIs below are illustrative
> fixtures only.

## Installation

`purrdf` is a native-only binary (it memory-maps pack files, so it is never built
for `wasm32`). Build it from the workspace:

```sh
cargo build --release -p purrdf-cli
# the binary is `purrdf`:
./target/release/purrdf --help
```

## Formats

Nine native RDF syntaxes plus the native pack container are accepted anywhere a
format is named (`--from`, `--to`, `--results-format`, or inferred from a path):

| Token | Syntax | Filename extensions |
|---|---|---|
| `turtle` (`ttl`) | Turtle | `.ttl` |
| `trig` | TriG | `.trig` |
| `ntriples` (`nt`) | N-Triples | `.nt` |
| `nquads` (`nq`) | N-Quads | `.nq` |
| `rdfxml` (`rdf`, `xml`) | RDF/XML | `.rdf`, `.xml` |
| `trix` | TriX | `.trix` |
| `hextuples` (`hext`) | HexTuples | `.hext` |
| `jsonld` (`json-ld`) | JSON-LD | `.jsonld` |
| `yamlld` (`yaml-ld`) | YAML-LD | `.yamlld` |
| `pack` | PurRDF pack container | `.purrpck`, `.pack` |

**Format inference.** When `--from`/`--to` is omitted, the format is inferred
from the path's extension. An explicit `--from`/`--to` always wins over the
extension.

**stdin/stdout.** A path of `-` reads from stdin or writes to stdout. Because `-`
has no extension, it **requires** an explicit `--from` (for input) or `--to` (for
output). `convert` defaults both `IN` and `OUT` to `-`.

**The pack container.** A pack is PurRDF's native, lossless RDF 1.2 container. On
disk it is opened **read-only and memory-mapped**, verified end-to-end
(`verify_pack`, fail-closed), and handed to the engine zero-copy — no intermediate
materialization for `convert` passthroughs, `query`, or serialization. A pack
arriving on stdin is read into a buffer and verified the same way. A `pack → pack`
`convert` is a verified byte passthrough (no decode/re-encode churn).

## `convert`

```text
purrdf convert [--from <F>] [--to <F>] [--base <IRI>] [--entailment <R>] [--canonical] [IN] [OUT]
```

Transcode a source into a target syntax or the pack container.

- `--from <F>` / `--to <F>` — input/output format overrides; inferred from the
  `IN`/`OUT` extension when omitted.
- `--base <IRI>` — base IRI for resolving relative IRIs while parsing, also
  threaded into the serializer as its base.
- `--entailment <R>` — materialize a regime's closure **in memory** before
  serializing (see [`reason`](#reason) for the seven regimes and their inputs).
- `--rules <FILE>` — the RIF-in-XML rule document `--entailment rif` runs;
  required by that regime and a usage error for any other.
- `--canonical` — emit the RDFC-1.0 canonical N-Quads document instead of `--to`.
  Canonical output is **always** N-Quads, so `--canonical` overrides (and lets you
  omit) `--to`.

Transforms compose in a fixed order: entail first, then canonicalize.

```sh
# Turtle → N-Triples, formats inferred from the extensions.
purrdf convert people.ttl people.nt

# JSON-LD on stdin → Turtle on stdout (explicit formats required for `-`).
cat people.jsonld | purrdf convert --from jsonld --to turtle - -

# Pack a graph into the native lossless container, then unpack it.
purrdf convert people.ttl people.purrpck
purrdf convert people.purrpck restored.trig

# Emit RDFC-1.0 canonical N-Quads (no `--to` needed; canonical is always N-Quads).
purrdf convert --canonical people.ttl people.nq

# Materialize the RDFS closure, then canonicalize it.
purrdf convert --entailment rdfs --canonical people.ttl closure.nq

# Resolve relative IRIs against a base while converting.
purrdf convert --base http://example.org/ data.ttl data.nt
```

## `query`

```text
purrdf query --data <file|pack> [--base <IRI>] [--entailment <R>] [--results-format <FMT>]
             [--fuel <N>] [--deadline <D>] [--max-answers <N>] [--max-intermediate-cells <N>]
             [--max-scratch-bytes <N>] [--max-remote-requests <N>] [--explain] '<SPARQL>'
```

Evaluate a SPARQL 1.2 query over a data source. The source is opened as a view (a
pack is queried **zero-copy**); the query text and the parsed data both resolve
relative IRIs against `--base`.

- `--data <file|pack>` — the data source (format inferred from its extension).
- `--base <IRI>` — base IRI applied to both the data parse and the query text.
- `--entailment <R>` — reconstruct an owned dataset, materialize the regime's
  closure in memory, and run the query over **the closure** (a pack is rebuilt for
  this; the zero-copy path is used only without `--entailment`).
- `--rules <FILE>` — the RIF-in-XML rule document `--entailment rif` runs;
  required by that regime and a usage error for any other.
- `--results-format <FMT>` — the result serialization (default `json`).

The **result shape** selects which half of `--results-format` is legal:

- **SELECT / ASK** produce solutions / a boolean → a SPARQL-results format:
  `json`, `xml`, `csv`, `tsv`.
- **CONSTRUCT / DESCRIBE** produce a graph → one of the nine RDF syntaxes
  (`turtle`, `trig`, `ntriples`, `nquads`, `rdfxml`, `trix`, `hextuples`,
  `jsonld`, `yamlld`).

A shape/format mismatch (e.g. SELECT solutions with `turtle`, or a CONSTRUCT graph
with `csv`) is a hard runtime error (exit 1). Results always go to stdout.

A **quad-template `CONSTRUCT`** (`CONSTRUCT { GRAPH ?g { … } }`, or the
whole-template `CONSTRUCT GRAPH <iri> { … }` shorthand) narrows that further. Only
six of the nine RDF syntaxes can carry a graph name — `trig`, `nquads`, `trix`,
`hextuples`, `jsonld`, `yamlld`. The other three (`turtle`, `ntriples`, `rdfxml`)
have no named-graph construct, and the single-graph serializers **drop** every
graph-scoped statement rather than folding it into the default graph. So a result
carrying any named graph, asked for one of those three, is a **usage refusal (exit
2)** whose stderr names the graphs, the format, and the quad-capable alternatives —
never a well-formed document silently missing what the query asked for. A mixed
template (default-graph triples plus named-graph quads) is refused as a whole for
the same reason; a result with only default-graph statements — every SPARQL 1.1
`CONSTRUCT`, every `DESCRIBE` — serializes to `turtle` unchanged.

### Execution governors

Six flags bound what one query is allowed to cost. Each is optional and each bounds
exactly the dimension it names; a dimension no flag names stays unbounded, and a
query with no governor flag runs the ungoverned path unchanged.

| Flag | Bounds | Unit |
|---|---|---|
| `--fuel <N>` | abstract execution steps | the engine's charge schedule (`--explain` prints it) |
| `--deadline <D>` | wall-clock **evaluation** time | count+unit components over `ms`, `s`, `m`, `h` — `750ms`, `30s`, `1m30s`, `2h` |
| `--max-answers <N>` | the query form's **answer sequence** | solution rows for SELECT; **output statements** — triples plus RDF 1.2 reifier bindings and annotations — for CONSTRUCT/DESCRIBE |
| `--max-intermediate-cells <N>` | the largest intermediate bag | cells (`rows × columns`) |
| `--max-scratch-bytes <N>` | the per-query scratch arena | bytes |
| `--max-remote-requests <N>` | `SERVICE` requests | requests |

Every ceiling is **inclusive**, so `0` is a valid one that trips at the first charge.
`--max-answers` is an operational ceiling and never `LIMIT`: `LIMIT` is query
semantics and applies before the cap is tested. `--max-intermediate-cells` is also
checked against the planner's *estimate* before evaluation begins, so a plan
predicted to exceed it is refused rather than started. On a raw query, `--deadline`
starts after the data and query have been read and parsed; with `--entailment`, it also
covers closure materialization. The evaluator observes it at operator entry and exit,
at logical charge points, and around a federated request; it is not a timeout on the
process. `--max-remote-requests` is
enforced and reported like any other ceiling, and this binary configures no
federation source, so a `SERVICE` clause fails to evaluate before it can be charged.

**A tripped governor is not a failure.** The run did exactly what it was told, so it
exits **3** rather than 1, and it writes three things:

- **stdout** — the answers the run certified, in the requested `--results-format`, as
  a *well-formed document of that format*. `purrdf query … | jq` keeps working across
  a trip.
- **stderr** — the deterministic governor report: `tripped` (the governor), `detail`,
  `answers` (`certain` / `at-most` / `withheld`), `positional-prefix`, and the whole
  `consumed`/`limit` vector, one line each.
- **exit 3** — the only thing a shell can test to learn that the document on stdout is
  a partial answer.

The partial status is never marked in-band, because a marker would corrupt the
SPARQL-Results stream (or require inventing a non-W3C extension to four
serializations). `answers certain` licenses the rows as answers; `answers at-most`
licenses only the negative reading; `answers withheld` means no bound survived the
plan, so *no row is printed at all* and a `barrier` line names the operator that
withheld them — printing an empty result there would be an "there are no answers"
claim the run cannot make.

`--entailment` accepts the same six flags. Every numeric ceiling bounds the SPARQL
evaluation over the completed closure. `--deadline` additionally reaches closure
materialization through its stop signal: a stopped closure returns no model and no
query result, while a completed closure is evaluated through the ordinary governed
query outcome. Numeric ceilings deliberately do not truncate a reasoning closure.

### `--explain`

`--explain` prints what the engine does with the query and what it costs — the charge
schedule it was priced under (with the profile's identity and digest), one line per
algebra node with the planner's estimate beside the cardinality that materialized, the
cost-based join orders, and the per-dimension consumption — *instead of* the answers,
the way `EXPLAIN` replaces a result set. The output is byte-deterministic for a given
query, dataset and build. It is a plain-text rendering on stdout, so `--results-format`
— which names how *answers* serialize, and an explanation has none — does not apply to
it.

It **evaluates** the query to produce that, under the metering profile: every counter
engaged at a ceiling nothing can reach. That is why it refuses a governor flag or
`--entailment` (exit 2) rather than accepting one it cannot honor — a ceiling it
reported but did not enforce would be worse than no ceiling at all.

```sh
# Cap the answer sequence; read the certified prefix and the reason it stopped.
purrdf query --data people.ttl --max-answers 100 --results-format json \
  'SELECT ?p ?name WHERE { ?p <http://example.org/name> ?name }' > page.json
case $? in
  0) echo "complete" ;;
  3) echo "partial — see the governor report above" ;;
esac

# Bound a query three ways at once.
purrdf query --data people.ttl --fuel 5000000 --deadline 30s --max-intermediate-cells 1000000 \
  'SELECT * WHERE { ?s ?p ?o }'

# Explain the plan and its charge ledger instead of answering.
purrdf query --data people.ttl --explain \
  'SELECT ?name WHERE { ?p <http://example.org/knows> ?q . ?q <http://example.org/name> ?name }'
```

```sh
# SELECT → SPARQL Results JSON (the default).
purrdf query --data people.ttl \
  'SELECT ?name WHERE { ?p <http://example.org/name> ?name }'

# ASK → CSV.
purrdf query --data people.ttl --results-format csv \
  'ASK { ?p <http://example.org/name> "Alice" }'

# CONSTRUCT → Turtle (a graph result serialized through an RDF syntax).
purrdf query --data people.ttl --results-format turtle \
  'CONSTRUCT { ?p <http://example.org/label> ?name } WHERE { ?p <http://example.org/name> ?name }'

# Query a pack zero-copy (mmap'd, verified, no materialization).
purrdf query --data people.purrpck --results-format tsv \
  'SELECT * WHERE { ?s ?p ?o } LIMIT 10'

# Query the RDFS closure rather than the raw graph.
purrdf query --data people.ttl --entailment rdfs \
  'SELECT ?type WHERE { <http://example.org/alice> a ?type }'
```

## `reason`

```text
purrdf reason --regime <R> [--rules <FILE>] [--from <F>] [--to <F>] [--base <IRI>] [IN] [OUT]
```

Materialize an entailment regime's closure over the source graph and write it out.

- `--regime <R>` — the entailment regime to close under.
- `--rules <FILE>` — the RIF-in-XML rule document `--regime rif` runs; required by
  that regime and a usage error for any other (see below).
- `--from <F>` / `--to <F>` — input/output format overrides; inferred from the
  `IN`/`OUT` extension when omitted. `IN`/`OUT` default to `-` (stdin/stdout); a
  path of `-` has no extension, so it **requires** the matching explicit
  `--from`/`--to`.
- `--base <IRI>` — base IRI for the input parse, also threaded into the serializer.

**Regimes the CLI materializes — all seven. None is refused.**

| `--regime` | Meaning | `--rules` |
|---|---|---|
| `simple` | Simple entailment (a faithful copy of the source) | — |
| `rdf` | RDF entailment | — |
| `rdfs` | RDFS entailment | — |
| `owl-rl` | OWL 2 RL entailment | — |
| `d` | Datatype entailment: Simple plus the five `dt-*` rules of OWL 2 Profiles §4.3 Table 8 | — |
| `owl-direct` | The SHOIQ(D) hypertableau's query-independent augmentation | — |
| `rif` | RIF-Core entailment under the rule document `--rules` names | **required** |

`rdfs` fires 18 of the 18 RDF + RDFS patterns; `owl-rl` fires all 78 rules of
OWL 2 Profiles §4.3 Tables 4–9. That is *rule-table coverage*, which is not
entailment conformance: on this vendored W3C corpus of OWL 2 RL entailment tests
this chase scores
**27 of 27 positive and 23 of 23 negative**, the latter meaning no unsoundness was
found. Both numbers are true and stating only the first is an overclaim; see
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md).

One further rule fires under `owl-rl` that no specification table states:
`ext-eq-diff-sym`, symmetry of `owl:differentFrom`. It is counted in neither
number above, and every report names it on an `extension` line, so a run whose
conclusions must be strictly normative can be filtered from the report itself.
The per-rule table is generated from the library's own API and drift-guarded:
[`docs/book/src/entailment-rules.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/book/src/entailment-rules.md).

**`--rules <FILE>` is a regime's own input, not an option.** `rif` entails under
the *caller's* rule set, which PurRDF does not declare, so it requires a normative
RIF-in-XML rule document; every other regime's rule table is the specification's,
so passing `--rules` to one is a **usage error** (exit 2) rather than a silently
discarded argument. An `Import` directive inside the document is refused by name:
this pipeline fetches nothing the operator did not name.

`owl-direct` needs no flag, and that is a statement rather than an omission. Its
extra input is a *query's* class expressions; `reason` and `convert` transform a
document and have no query, so what runs is the query-independent augmentation —
the classification, the realization, the entailed role assertions and the
`owl:sameAs` identifications the tableau decides about the ontology's own named
terms.

`convert --entailment` and `query --entailment` share this resolver and accept
`--rules` identically.

```sh
# Materialize the RDFS closure and write it as N-Triples.
purrdf reason --regime rdfs people.ttl closure.nt

# OWL 2 RL closure from stdin to stdout (explicit formats required for `-`).
cat ontology.ttl | purrdf reason --regime owl-rl --from ttl --to nt - -

# Every regime materializes, including the two `entails` refuses.
purrdf reason --regime owl-direct people.ttl out.ttl
echo $?   # 0
```

## `entails`

```text
purrdf entails --regime <R> --premise <FILE> (--conclusion <FILE> [--verify] | --pattern <FILE>)
               [--import <IRI>=<FILE>]... [--report[=PATH]] [--from <F>] [--base <IRI>] [OUT]
```

Decide a question about a premise, rather than compute everything it entails.

`reason` and `entails` are the two halves of entailment and neither is the other.
`reason` builds a **closure** — everything the premise entails, as a document —
which is what you want if you are going to ask many questions of one premise.
`entails` decides **one** question, and that is not the membership test in that
closure it looks like: a conclusion's blank nodes are existentials that have to be
*mapped*, an inconsistent premise entails everything, and a failure to find a
mapping means nothing at all unless the rule set is complete for the premise it
ran on. A conclusion can also be entailed while appearing nowhere in the closure
— see the mechanisms below.

- `--regime <R>` — the regime to decide under (see the refusal note below).
- `--premise <FILE>` — the premise document, or `-` for stdin.
- `--conclusion <FILE>` — a conclusion **graph**. The answer is a verdict.
- `--verify` — re-decide the warrant of an `entailed` verdict **without running a
  reasoner**, adding `warrant` and `verified` lines. Requires `--conclusion`.
- `--pattern <FILE>` — a **basic graph pattern**: N-Triples with `?name` (or
  `$name`) in any position, the **predicate** included. The answer is its *certain
  answers* — the substitutions the premise entails the pattern under, not the ones
  that happen to be in one closure. A pattern is not an RDF document, so its bytes
  go to the boundary untranscoded and `--from` says nothing about it. A predicate
  variable is projected like any other, and under `owl-rl` it also renders a
  `limit`: it ranges over the whole predicate vocabulary, including the schema
  predicates and the constructs the mechanisms beyond the rule table decide, and the
  closure the rows are drawn from holds neither. The one slot that admits no
  variable is a literal's **datatype**: `"5"^^?d` asks for a binding in a position
  that holds an IRI rather than a term, and is refused by name.
- `--import <IRI>=<FILE>` — repeatable; resolves one `owl:imports` the premise
  declares to one local document (see below).
- `--report[=PATH]` — the reasoning certificate, as `reason --report`.
- `--from <F>` / `--base <IRI>` — the input format override and parse base for the
  premise, the conclusion and every `--import` document.

`--conclusion` and `--pattern` conflict and exactly one is required. The answer
goes to `OUT` (default stdout) and the certificate to `--report`, so the two never
mix even when `OUT` is `-`.

**Three verdicts, never two.** `entailment not-entailed` is a *proof* — the
procedure was complete for this premise, so the absence of a mapping is the
absence of an entailment — and `entailment undecided` is what an incomplete
procedure says instead. Collapsing the second into the first would turn a
limitation of the library into a false statement about your data.

**Six mechanisms, and the answer names the one that reached it.** The first line
of every answer is `mechanism <name>`:

| `mechanism` | Reaches |
|---|---|
| `strict-table` | The regime's own rule table, run once. The only mechanism a `not-entailed` can carry: refuting needs the completeness half of a theorem, and only the table has one. |
| `refutation` | A **negative fact** (`owl:differentFrom`, membership in an `owl:complementOf` class). No head in OWL 2 Profiles §4.3 Tables 4–9 has that shape, so the seventeen `false`-concluding rules decide it instead. |
| `freeze` | A **schema axiom** (a property characteristic, an inclusion). Theorem PR1 claims completeness only for *assertional* conclusions, so an absent schema triple is not a fact about the premise — and that holds for an inclusion too, which Table 9's `scm-*` rules do conclude. |
| `comprehension` | An **anonymous class expression** the conclusion names, under the RDF-Based comprehension conditions. |
| `reflexivity` | A conclusion's **self-loops**, read off the premise's `owl:ReflexiveProperty` typings. `owl:ReflexiveProperty` is outside the OWL 2 RL syntax. |
| `data-range` | A **containment between value spaces** (`xsd:byte ⊑ xsd:short`), which no join over triples can discover. |
| `composite` | Two or more of the above, folded over one conclusion — a conclusion graph is a conjunction, so it can need a lane per half. The `constituent` lines name which. |

None of the five beyond `strict-table` adds a rule: `rules()`, `implemented()` and
`extensions()` are untouched by all of them, and a `reason` closure is byte-for-byte
what it was.

**Five regimes, and the other two are refused by name.** `--regime` takes the same
seven spellings `reason` does, because the accepted set is one vocabulary across
the binary. This question is served by five of them. `owl-direct` is directed by a
*query's* class expressions and `rif` entails under the *caller's* rule document,
and "premise, conclusion, regime" carries neither — so both are refused, naming
the regime (exit 1), rather than answered under a weaker regime and labelled with
the one you asked for. Both still **materialize**: `purrdf reason --regime
owl-direct` and `purrdf reason --regime rif --rules FILE` carry the input each is
defined by.

**`--import <IRI>=<FILE>` is configuration, not a fetch.** OWL 2 defines an
ontology's imports closure to *be* the ontology, so a premise carrying an
`owl:imports` this command was not handed is a different premise from the one you
asked about. **PurRDF fetches nothing and mints no vocabulary**, so each pair
resolves one ontology IRI to one local document; an `owl:imports` no pair resolves
is refused by name (exit 1) rather than treated as an empty document, and a
malformed pair (no `=`) is a usage error (exit 2) rather than a skipped import. The
IRI is everything before the *first* `=`.

**One stdin.** The premise, the question and each `--import` document may each be
`-`, and at most one of them may be: a process has a single standard input, so two
documents reading it would each get part of one. Two is a usage error naming both.

**No document flags.** `--loss-ledger` records what a conversion dropped and
`--jsonld-options` configures an RDF serializer; `entails` writes a verdict, so
both are refused (exit 2) rather than silently doing nothing. The documents it
reads cross into the boundary's N-Quads — the syntax that carries named graphs, the
RDF 1.2 statement layer and literal base direction — so that crossing loses nothing
from any of the nine, and a realized drop is refused rather than recorded.

```sh
# Does the ontology entail the conclusion under OWL 2 RL?
purrdf entails --regime owl-rl --premise ontology.ttl --conclusion claim.ttl
# mechanism strict-table
# entailment entailed

# A negative fact: entailed, and nowhere in the closure.
purrdf entails --regime owl-rl --premise family.ttl --conclusion different.ttl
# mechanism refutation
# entailment entailed

# Re-decide the warrant without running a reasoner.
purrdf entails --regime owl-rl --premise ontology.ttl --conclusion claim.ttl --verify
# … warrant present / verified true

# A premise that imports its schema, plus the certificate on stderr.
purrdf entails --regime owl-rl --premise ontology.ttl --conclusion claim.ttl \
  --import http://example.org/schema=schema.ttl --report

# The certain answers of a basic graph pattern.
purrdf entails --regime rdfs --premise people.ttl --pattern types.bgp
```

## `consistency`

```text
purrdf consistency [--step-cap <N>] [--work-cap <N>] [--from <F>] [--base <IRI>] [IN]
```

Decide whether an OWL-Direct ontology has a model at all. This is the one DL question
`reason` and `entails` cannot reach: `reason --regime owl-direct` **refuses** an
inconsistent ontology outright (an inconsistent knowledge base entails every triple, so
there is no closure to materialize), and `entails` decides a conclusion against a premise
that is presupposed to have a model. Neither can answer "does this ontology have a model
at all", because both are built on top of an answer to that question rather than able to
give it. `consistency` asks it directly, through the same string boundary the Python,
WebAssembly and C-ABI hosts already reach, so a verdict this binary prints is byte-for-byte
the verdict those three print for the same document.

- `--step-cap <N>` — narrows the per-decision **round** cap the ontology's own size already
  derives; `0` (the default) applies no narrowing and runs under the derived cap alone.
  This can only **tighten** the cap, never loosen it: a run this narrows into its cap
  answers `unknown`, never `false`.
- `--work-cap <N>` — narrows the per-decision **work** cap the ontology's own size already
  derives, on the same `0`-means-no-narrowing, tighten-only rule. It bounds what
  `--step-cap` structurally cannot: a round is a PASS over the completion graph rather than
  a unit of cost, so an ontology can make every round enormously more expensive without
  making the search take more rounds — one individual co-typed with several
  equivalence-defined classes does exactly that. This cap counts the matcher, scan, closure
  and clone work spent inside a round.
- `--from <F>` — input-format override; inferred from `IN`'s extension when omitted.
- `--base <IRI>` — base IRI for resolving relative IRIs while parsing the input.
- `IN` — the ontology, or `-` for stdin (which requires `--from`); defaults to `-`.

The boundary parses N-Quads (which accepts N-Triples unchanged), so a caller handing this
command Turtle, RDF/XML, JSON-LD or a verified pack crosses into it exactly as `entails`
crosses its premise: resolved through `--from`/the path's extension, parsed with the native
codecs, and re-serialized into N-Quads. That crossing is lossless by construction, and a
**realized** drop is refused rather than recorded, because a lossily transcoded ontology is
a different ontology and the verdict would be about that one.

**Two things go to stdout, always.** The one-line verdict —

```text
consistency true | false | unknown
```

— followed immediately by the full DL certificate: `completeness`, the reverse mapping's
boundary list, and eight search-cost counters. Unlike `--report` on the four materializing
subcommands, the certificate here is **not optional and not redirectable**: this command
answers exactly one question, and the certificate is the *second half* of that answer, not
secondary evidence about a document sitting beside it — hiding it behind a flag would
restore the "the reasoner says no" ambiguity the certificate exists to remove.

| Certificate line | What it counts |
|---|---|
| `steps` | rounds spent, against the per-decision round cap |
| `budget` | the round cap the decision ran under (derived, or narrowed by `--step-cap`) |
| `work` | matcher, scan, closure and clone work spent, against the work cap |
| `work-budget` | the work cap the decision ran under (derived, or narrowed by `--work-cap`) |
| `decisions` | how many sub-decisions the run made |
| `peak-nodes` | the largest completion graph a decision built |
| `disjunctions` | how many times the `⊔`-rule case split |
| `peak-depth` | how deep that rule's branch stack got |

**No document flags.** `--loss-ledger` records what a conversion dropped and
`--jsonld-options` configures an RDF serializer; `consistency` writes a verdict plus a
certificate, neither of which is RDF, so both are refused (exit 2) rather than silently
doing nothing.

```sh
# An ordinary ontology decides well inside its derived budgets.
purrdf consistency ontology.ttl
# consistency true
# completeness decided
# …

# Narrow the round cap to force an early `unknown` rather than let the search run to
# its derived budget.
purrdf consistency --step-cap 1 ontology.ttl
echo $?   # 3

# A pack source: verified, then reverse-mapped into the tableau like any other input.
purrdf consistency ontology.purrpck
```

## `validate`

```text
purrdf validate --shapes <FILE> [--shapes-from <F>] [--shapes-graph <IRI>]
                [--from <F>] [--base <IRI>] [--format <F>]
                [--fuel <N>] [--deadline <D>] [--max-intermediate-cells <N>]
                [--max-scratch-bytes <N>] [--max-remote-requests <N>]
                [IN] [OUT]
```

Validate an RDF data graph against a SHACL shapes graph — full SHACL Core plus
SHACL-SPARQL and SHACL-AF, over the same `purrdf-shapes` engine the Rust,
Python, WebAssembly and C surfaces reach.

**The answer is the W3C validation report.** The SHACL specification defines the
validation process to produce a validation *report* — an RDF graph of
`sh:ValidationResult` nodes under a `sh:ValidationReport` — and that graph is
what `--format` serializes, defaulting to `ntriples`. Being ordinary RDF, it
composes with the rest of this binary: a report can be queried, transcoded, or
itself validated.

| `--format` | Artifact |
|---|---|
| `ntriples` (default), `turtle`, `trig`, `nquads`, `rdfxml`, `trix`, `hextuples`, `jsonld`, `yamlld` | the SHACL results **graph** in that syntax |
| `sarif` | SARIF 2.1.0 JSON, for an editor or a code-scanning dashboard |

SARIF is a *projection* of the same report rather than a second answer: several
SHACL severities collapse onto SARIF's three levels (the verbatim severity IRI
survives only in a property bag) and the RDF term structure becomes strings.
That is exactly right for a CI consumer and exactly wrong as the artifact
everything else derives from — hence an opt-in, not the default. Both come from
one engine run and one writer (`purrdf_validate::report_to_sarif_string`, the
same one the WASM and C-ABI hosts use), so they can never disagree.

**The verdict is always on stderr.** stdout carries a well-formed RDF or SARIF
document, which it could not if the verdict were interleaved into it, so two
`key value` lines — `shacl conforms true|false` and `shacl results N` — go to
stderr on every run. A shell branches on those without parsing the artifact.

**Exit codes.** `0` whether the data conforms **or not** — both are decided
verdicts, exactly like `consistency true|false` and a `false` ASK; `1` for a
malformed document or an unsupported/structurally incomplete SHACL construct
(hard-failed, never silently skipped); `2` for a usage error; `3` when a
governor stopped the run — and then **no report is written at all**, because
every SHACL constraint is a negative claim and a truncated solution bag cannot
license a `conforms`.

**Inputs.** The data graph is any of the nine syntaxes or a verified pack,
resolved by `--from`/extension. The shapes graph is resolved by
`--shapes-from`/extension; Turtle takes the shared `parse_shapes` boundary (the
one that also recovers the document's `@prefix` map as the fallback prefix
environment for SHACL-AF `sh:select`), every other syntax is parsed by the
native codec. `--shapes-graph <IRI>` exposes the shapes graph to SHACL-SPARQL
paths as a named graph; there is no default IRI, because PurRDF mints no
vocabulary.

```sh
# The results graph, N-Triples on stdout, verdict on stderr.
purrdf validate --shapes shapes.ttl data.ttl

# SARIF for CI.
purrdf validate --shapes shapes.ttl --format sarif data.ttl > results.sarif

# A verified pack data source and a Turtle report file.
purrdf validate --shapes shapes.ttl --format turtle data.purrpck report.ttl

# Query the report the binary just produced.
purrdf validate --shapes shapes.ttl data.ttl report.nt
purrdf query --data report.nt --results-format csv \
  'SELECT ?focus WHERE { ?r <http://www.w3.org/ns/shacl#focusNode> ?focus }'

# Bound the SHACL-SPARQL paths; a trip writes no report and exits 3.
purrdf validate --shapes shapes.ttl --deadline 5s data.ttl
```

## `shex`

```text
purrdf shex --schema <FILE> [--schema-from <shexc|shexj>] [--import <IRI=FILE>]…
            --data <FILE> [--from <F>] [--base <IRI>] MAP [OUT]
```

Validate RDF nodes against a **ShEx 2.1** schema through a query shape map. The
schema is ShExC or ShExJ (resolved by `--schema-from`, or from the `.shex` /
`.shexc` / `.shexj` / `.json` extension); `MAP` is the ShapeMap specification's
compact syntax, so a node is `<iri>` / `_:label` / a Turtle literal / a
triple-pattern selector (`{FOCUS <p> _}`, `{FOCUS a <C>}`, `{_ <p> FOCUS}`) and
a shape is `START` or `<label>`.

The answer is the ShapeMap specification's **result shape map**: a JSON array of
`{"node","shape","status","reason"?}` objects in the engine's own deterministic
order. That is `purrdf-shex`'s single rendered form, so there is no `--format`
to choose. As with `validate`, both verdicts exit `0` and the summary — `shex
conformant true|false`, `shex entries N`, `shex nonconformant N` — goes to
stderr.

**Three constructs are refused by name rather than answered weakly.** Each has a
documented library fallback that is honest as a *behavior* and a lie as a
printed *verdict*:

| Construct | Why it is refused (exit `1`) |
|---|---|
| an unresolved `IMPORT` | the imported labels would dangle, so the validated schema is not the one you wrote. Supply it with `--import IRI=FILE` — PurRDF fetches nothing. A pair the import closure never reaches is refused too, rather than read and unused. |
| an `EXTERNAL` shape | with no resolver it "fails every node" — a definite `nonconformant` derived from semantics nobody supplied. A resolver is host code, not a document. |
| a semantic action | the empty extension registry makes every action an *inert success*, which would report a conformance the check never granted. |

**RDF 1.2.** A triple term is an ordinary node: it matches an arc's object and
may be named as a focus node (`<< s p o >>` in the result map). The RDF 1.2
*statement layer* (a `{| … |}` reifier and its annotations) is read too: a
selector over an annotation predicate selects the reifier carrying it, and a
reifier focus node's neighbourhood is the union of its ordinary arcs, its
`rdf:reifies` arc, and its annotations. ShEx 2.1 predates RDF 1.2 and describes
only arcs; PurRDF extends the data model rather than inheriting the gap, so
`shex`, `validate` (SHACL) and `query` (SPARQL) all answer alike.

```sh
# One fixed association.
purrdf shex --schema user.shex --data data.ttl \
  '<http://example.org/alice>@<http://example.org/UserShape>'

# Every node typed ex:Person, selected by a query shape map.
purrdf shex --schema user.shex --data data.ttl \
  '{FOCUS a <http://example.org/Person>}@<http://example.org/UserShape>'

# A ShExJ schema with a caller-resolved IMPORT, over a verified pack.
purrdf shex --schema user.shexj --import http://example.org/ages=ages.shex \
  --data data.purrpck '<http://example.org/alice>@<http://example.org/UserShape>'
```

## `describe`

```text
purrdf describe --iri <IRI>… [--from <F>] [--to <F>] [--base <IRI>] [IN] [OUT]
```

Extract the **Symmetric Concise Bounded Description** of one or more resources:
outgoing *and* incoming arcs, the transitive blank-node closure in both
directions, and the RDF 1.2 statement layer — the reifiers whose reified triple
touches the closure, together with their annotations. Named-node neighbours do
not expand. Several `--iri` values are described as one union subgraph.

This is the same `purrdf_core::describe::Describer` that SPARQL `DESCRIBE`
evaluates to in this engine, reached rather than re-derived: `purrdf describe
--iri X` and `purrdf query 'DESCRIBE <X>'` produce byte-identical output. It is
a verb of its own for three reasons an operator meets immediately —
`query`'s `--results-format` defaults to `json`, which is illegal for a graph
result, so the obvious `purrdf query --data d.ttl 'DESCRIBE <x>'` hard-fails,
while `describe` resolves `--to`/the `OUT` extension like `convert` does; a
resource is an argument rather than SPARQL text a script has to build around it;
and being an RDF-emitting verb, `--loss-ledger` and `--jsonld-options` apply
exactly as they do to `convert`.

Describing a subject the source does not mention yields an empty description and
exit `0` — a term may legitimately carry no asserted or incoming triples.
Nothing here infers, so there is no `--report`; describe the output of `purrdf
reason` if you want the description of a closure.

```sh
# A resource's full symmetric description, format from the OUT extension.
purrdf describe --iri http://example.org/alice data.ttl alice.ttl

# Two subjects as one union subgraph, on stdout.
purrdf describe --iri http://example.org/alice --iri http://example.org/bob \
  --to ntriples data.ttl

# From a verified pack (zero-copy) into the lossless pack container.
purrdf describe --iri http://example.org/alice source.purrpck alice.purrpck

# A description carrying RDF 1.2 statement metadata into a star-incapable
# syntax: the dropped reifier rows are recorded, not lost silently.
purrdf --loss-ledger describe --iri http://example.org/alice --to rdfxml star.ttl
```

## `project`

```text
purrdf project --profile <P> --config <PATH> [--from <F>] [--base <IRI>] [IN] [OUT]
```

Project an RDF syntax or verified pack source into one canonical USTAR archive.
The mandatory JSON configuration is tagged with the same profile and supplies
all vocabulary, package identity, resource limits, and processing policy. A
profile/config mismatch, an unknown field, or a breached limit is a hard error.

| Profile | Native view | Liftable |
| --- | --- | :---: |
| `lpg-csv` | Generic nodes/edges CSV | yes |
| `neo4j-csv` | Neo4j Admin Import CSV | yes |
| `open-cypher` | Closed deterministic `CREATE` grammar | yes |
| `graphml` | GraphML 1.0 | yes |
| `csvw-exact` | Exact RDF 1.2 CSVW table group | yes |
| `csvw-terms` | Caller-declared curated CSVW entity tables | no |
| `okf-terms` | Caller-declared OKF v0.1 concept bundle | no |
| `obo-graphs` | OBO Graphs 0.3.2 JSON | no |
| `skos` | SKOS Turtle concept-scheme view | no |
| `croissant-1.1` | Croissant 1.1 JSON-LD | yes |
| `ro-crate-1.3` | RO-Crate 1.3 JSON-LD | yes |
| `datacite-4.6` | DataCite 4.6 XML | yes |
| `dcat-3` | DCAT 3 JSON-LD research-object carrier | yes |
| `dcat-rdf` | Mapped or caller-CONSTRUCTed native DCAT RDF | no |
| `void` | VoID statistics, partitions, and linksets in native RDF | no |
| `frictionless-data-package-1` | Frictionless Data Package v1 JSON | yes |

A minimal generic LPG configuration is:

```json
{
  "profile": "lpg-csv",
  "config": {
    "rdf_type": "https://example.org/type",
    "scope": {"mode": "all"},
    "limits": {
      "max_artifacts": 16,
      "max_artifact_bytes": 1000000,
      "max_total_bytes": 4000000,
      "max_archive_bytes": 5000000,
      "max_term_depth": 16
    },
    "execution_limits": {
      "max_input_records": 1000,
      "max_model_records": 1000,
      "max_nodes": 1000,
      "max_edges": 1000
    }
  }
}
```

```sh
purrdf --loss-ledger=project.loss.json project \
  --profile lpg-csv --config lpg.json --from turtle \
  graph.ttl graph.tar
```

The archive bytes are deterministic for the same dataset and configuration.
LPG profiles retain exact RDF sideband for reconstruction, while the semantic
lowering into a property graph remains visible in the ledger. `csvw-exact` is
lossless. Curated CSVW/OKF terms, OBO Graphs, SKOS, native DCAT RDF, and VoID
are intentionally write-only views.

The two native RDF dataset-description profiles accept the same complete
caller-owned configurations in every host. `dcat-rdf` selects either the shared
mapped research-object model or a bounded whole-dataset CONSTRUCT. `void`
selects exact source graphs and emits bounded statistics, partitions, and
oriented linksets using caller-supplied role IRIs and dataset prefixes:

```sh
purrdf project --profile dcat-rdf --config dcat-rdf.json \
  --from trig source.trig dcat.tar
purrdf project --profile void --config void.json \
  --from trig source.trig void.tar
```

Runnable `example.org` inputs are in
`crates/rdf/tests/fixtures/dataset-description/`. The resulting archives contain
one `dcat.<extension>` or `void.<extension>` member selected by the configured
native syntax. `examples/dataset-descriptions.sh` executes both profiles twice
and verifies their archive bytes are identical.

## `lift`

```text
purrdf lift --profile <P> --config <PATH> --to <F> [--base <IRI>] [IN] [OUT]
```

Lift one canonical archive into a native RDF syntax. The accepted profiles are
`lpg-csv`, `neo4j-csv`, `open-cypher`, `graphml`, `csvw-exact`,
`croissant-1.1`, `ro-crate-1.3`, `datacite-4.6`, `dcat-3`, and
`frictionless-data-package-1`. The CLI does not offer curated CSVW terms,
`okf-terms`, OBO Graphs, SKOS, native DCAT RDF, or VoID as pretend reverse mappings;
`purrdf lift --profile okf-terms` is rejected instead of fabricating one. The
reader rejects non-canonical USTAR, unexpected members, malformed carrier data,
sideband inconsistencies, and resource-limit violations.

```sh
purrdf --loss-ledger=lift.loss.json lift \
  --profile lpg-csv --config lpg.json --to nquads \
  graph.tar restored.nq
```

Configuration and archive input may independently use `-`, but not
simultaneously because stdin cannot supply both byte streams. A complete
runnable round trip lives in `examples/projection-roundtrip.sh`.

## The loss ledger

`--loss-ledger` is a global flag that surfaces the machine-readable loss record
for a conversion, projection, or lift. The ledger is **always computed**; the
flag only controls where (if anywhere) it is written, via three states:

| Form | Effect |
|---|---|
| absent | silent — the ledger is not surfaced |
| `--loss-ledger` (bare) | render the ledger's JSON to **stderr** |
| `--loss-ledger=PATH` | write the ledger's JSON to **PATH** |

The `=PATH` spelling is required (the bare form takes no value), so the flag never
swallows a following subcommand or query string.

For syntax conversion, the ledger records both the **contract** losses inherent
to a `(source-codec → target-codec)` pair and the **realized** counts the
serializer actually dropped. Projection ledgers use the same versioned schema
but add stable source locations for graph/tabular semantic lowering. A pack
target, a `pack → pack` passthrough, RDFC-1.0 canonical N-Quads, and
`csvw-exact` are lossless, so their ledgers are empty.

```sh
# Convert to a star-incapable syntax and inspect what was dropped, on stderr.
purrdf --loss-ledger convert star-data.ttl plain.rdf

# Persist the ledger to a file alongside the output.
purrdf --loss-ledger=convert.loss.json convert star-data.ttl plain.trix
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | success — including every **decided negative verdict** (see below) |
| `1` | runtime failure — a parse/serialize diagnostic, a pack-integrity failure, an I/O error, a result/shape mismatch, a refusal from the entailment boundary (an unserved regime, an unresolved `owl:imports`, an inconsistent premise), an unsupported or structurally incomplete SHACL construct, or a ShEx schema whose semantics this boundary cannot supply (an unresolved `IMPORT`, an `EXTERNAL` shape, a semantic action) |
| `2` | usage error — a malformed command line (clap), or a pipeline usage error such as `-` without an explicit format, `--regime rif` without `--rules`, a malformed `--import` pair, two documents reading stdin, or a flag that names something the selected mode does not produce |
| `3` | a caller-set [execution governor](#execution-governors) stopped a `query`, an `update` or a [`validate`](#validate); or [`consistency`](#consistency) answered `unknown`. **Not a failure**: for `query`, the certified answers are on stdout and the governor report is on stderr; for `update` and `validate`, nothing was produced (a mutation is atomic and a truncated SHACL run cannot license a verdict) and the receipt is on stderr; for `consistency`, the verdict and the full certificate — including which cap it was — are on stdout as always |

On any failure the error's message is printed to stderr and its category becomes
the process exit code; nothing is swallowed.

**A decided "no" is not an exit code.** A `false` ASK, a `not-entailed`
conclusion, an inconsistent ontology, a non-conforming SHACL data graph and a
nonconformant ShEx node all exit `0` and put the answer on stdout. The run did
exactly what it was asked to do, and mapping "the answer is no" onto a failure
code would put it in the same bucket as a corrupt pack — the flattening the
governor code exists to prevent, in the other direction. Because
[`validate`](#validate) and [`shex`](#shex) must keep stdout a well-formed RDF
or JSON document, each also writes its one-line verdict to **stderr** on every
run, so a shell can branch on conformance without parsing the artifact:

```sh
purrdf validate --shapes shapes.ttl data.ttl > report.nt 2> verdict.txt
grep -q 'shacl conforms true' verdict.txt || echo "shapes violated"
```

**Why a governor trip is its own code.** `1` would put a truncated answer in the same
bucket as a corrupt pack, so a pipeline could not tell "your query was cut short —
here is the certified prefix" from "your query failed and there is nothing to read".
`0` would be worse: a truncated answer reported as a complete one is silently wrong,
and every consumer downstream believes it. So a trip is neither, and it never travels
the error path — it is not printed with the `purrdf:` prefix, because nothing went
wrong.

There is still **no unsupported-regime exit code**, and that is a different question.
A third code used to classify the entailment-regime boundary *the CLI decided for
itself*; `purrdf-entail`'s `materialize` takes a `Materialization` — which carries each
regime's own input — so all seven **materialize** and the classification had nothing
left to classify. The two regimes [`entails`](#entails) does not serve are refused as
ordinary runtime failures (`1`) carrying the boundary's own diagnostic, which names the
regime: the CLI keeps no second list of which regimes that service serves, because a
second list is a second opinion. A budget trip is not a list the CLI keeps either — it
is an outcome the engine itself reports, in a type whose whole design says it is
neither a result nor an error, and an exit code is the only channel a process boundary
has for carrying that distinction.

## License

Licensed under either of [MIT](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
or [Apache-2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
at your option.
