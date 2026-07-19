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

Every invocation is one `Source → [transform] → Sink` pipeline, exposed as five
subcommands:

| Subcommand | Pipeline |
|---|---|
| [`convert`](#convert) | transcode RDF between syntaxes and the native pack container |
| [`query`](#query) | evaluate a SPARQL query over an RDF or pack data source |
| [`reason`](#reason) | materialize an entailment regime's closure over a source graph |
| [`project`](#project) | materialize a deterministic graph/tabular USTAR carrier |
| [`lift`](#lift) | reconstruct RDF from a strict bidirectional carrier |

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
  serializing (see [`reason`](#reason) for the supported regimes and the exit-3
  boundary; the two lanes reject identically).
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
purrdf query --data <file|pack> [--base <IRI>] [--entailment <R>] [--results-format <FMT>] '<SPARQL>'
```

Evaluate a SPARQL 1.2 query over a data source. The source is opened as a view (a
pack is queried **zero-copy**); the query text and the parsed data both resolve
relative IRIs against `--base`.

- `--data <file|pack>` — the data source (format inferred from its extension).
- `--base <IRI>` — base IRI applied to both the data parse and the query text.
- `--entailment <R>` — reconstruct an owned dataset, materialize the regime's
  closure in memory, and run the query over **the closure** (a pack is rebuilt for
  this; the zero-copy path is used only without `--entailment`).
- `--results-format <FMT>` — the result serialization (default `json`).

The **result shape** selects which half of `--results-format` is legal:

- **SELECT / ASK** produce solutions / a boolean → a SPARQL-results format:
  `json`, `xml`, `csv`, `tsv`.
- **CONSTRUCT / DESCRIBE** produce a graph → one of the nine RDF syntaxes
  (`turtle`, `trig`, `ntriples`, `nquads`, `rdfxml`, `trix`, `hextuples`,
  `jsonld`, `yamlld`).

A shape/format mismatch (e.g. SELECT solutions with `turtle`, or a CONSTRUCT graph
with `csv`) is a hard runtime error (exit 1). Results always go to stdout.

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
purrdf reason --regime <R> [--from <F>] [--to <F>] [--base <IRI>] [IN] [OUT]
```

Materialize an entailment regime's closure over the source graph and write it out.

- `--regime <R>` — the entailment regime to close under.
- `--from <F>` / `--to <F>` — input/output format overrides; inferred from the
  `IN`/`OUT` extension when omitted. `IN`/`OUT` default to `-` (stdin/stdout); a
  path of `-` has no extension, so it **requires** the matching explicit
  `--from`/`--to`.
- `--base <IRI>` — base IRI for the input parse, also threaded into the serializer.

**Supported (materializable) regimes:**

| `--regime` | Meaning |
|---|---|
| `simple` | Simple entailment (a faithful copy of the source) |
| `rdf` | RDF entailment |
| `rdfs` | RDFS entailment |
| `owl-rl` | OWL 2 RL entailment |

**The unsupported boundary (exit code 3).** Three regimes cannot be materialized
by the CLI because they need inputs it has no way to supply, and each is rejected
with a distinct diagnostic:

- `owl-direct` — OWL Direct (DL) needs the query's class expressions;
- `rif` — RIF-Core needs a parsed rule set;
- `d` — datatype (D) entailment is a spec-inherent materialization boundary.

`convert --entailment` shares this boundary and rejects identically.

```sh
# Materialize the RDFS closure and write it as N-Triples.
purrdf reason --regime rdfs people.ttl closure.nt

# OWL 2 RL closure from stdin to stdout (explicit formats required for `-`).
cat ontology.ttl | purrdf reason --regime owl-rl --from ttl --to nt - -

# The unsupported boundary: exits 3 with an explanatory message.
purrdf reason --regime owl-direct people.ttl out.ttl
echo $?   # 3
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
| `obo-graphs` | OBO Graphs 0.3.2 JSON | no |
| `skos` | SKOS Turtle concept-scheme view | no |

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
lossless. Curated CSVW terms, OBO Graphs, and SKOS are intentionally lossy views.

## `lift`

```text
purrdf lift --profile <P> --config <PATH> --to <F> [--base <IRI>] [IN] [OUT]
```

Lift one canonical archive into a native RDF syntax. The accepted profiles are
`lpg-csv`, `neo4j-csv`, `open-cypher`, `graphml`, `csvw-exact`,
`croissant-1.1`, `ro-crate-1.3`, `datacite-4.6`, `dcat-3`, and
`frictionless-data-package-1`. The CLI does not offer curated CSVW terms, OBO
Graphs, or SKOS as pretend reverse mappings. The reader rejects non-canonical
USTAR, unexpected members, malformed carrier data, sideband inconsistencies,
and resource-limit violations.

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
| `0` | success |
| `1` | runtime failure — a parse/serialize diagnostic, a pack-integrity failure, an I/O error, or a result/shape mismatch |
| `2` | usage error — a malformed command line (clap), or a pipeline usage error such as `-` without an explicit format |
| `3` | unsupported entailment regime — `owl-direct` / `rif` / `d` cannot be materialized by the CLI |

On any failure the error's message is printed to stderr and its category becomes
the process exit code; nothing is swallowed.

## License

Licensed under either of [MIT](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
or [Apache-2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
at your option.
