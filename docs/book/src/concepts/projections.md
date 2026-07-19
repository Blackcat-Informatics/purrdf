<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Graph, Tabular & Research-Object Projections

PurRDF projects an RDF 1.2 dataset into graph, tabular, and research-object
carrier formats without making any of those formats the semantic authority. One
Rust engine implements the mapping, packages its artifacts as canonical USTAR,
and is exposed unchanged through Rust, the CLI, Python, WebAssembly, and C.

Every operation has four non-negotiable properties:

- configuration supplies every vocabulary role, identity IRI, processing policy,
  and resource limit; the library fabricates none;
- output bytes are deterministic for the same dataset and configuration;
- loss is always computed as a closed, located ledger, even when a host chooses
  not to display it;
- malformed, ambiguous, non-canonical, or out-of-bounds input is a hard error.

## Profiles

| Profile | Native artifacts | Direction | RDF fidelity |
| --- | --- | --- | --- |
| `lpg-csv` | generic nodes/edges CSV | RDF ↔ carrier | exact RDF sideband; property-graph lowering ledgered |
| `neo4j-csv` | Neo4j Admin Import CSV | RDF ↔ carrier | same canonical LPG authority and ledger |
| `open-cypher` | deterministic `CREATE` program | RDF ↔ carrier | strict reader accepts exactly the emitted grammar |
| `graphml` | GraphML 1.0 XML | RDF ↔ carrier | exact RDF sideband; strict namespace/key validation |
| `csvw-exact` | CSVW metadata plus RDF 1.2 tables | RDF ↔ carrier | lossless |
| `csvw-terms` | CSVW metadata plus caller-declared entity tables | RDF → view | located, closed loss ledger |
| `obo-graphs` | OBO Graphs 0.3.2 JSON | RDF → view | located, closed loss ledger |
| `skos` | SKOS Turtle | RDF → view | located, closed loss ledger |
| `croissant-1.1` | `croissant.json` | RDF ↔ carrier | shared model; profile loss is located |
| `ro-crate-1.3` | `ro-crate-metadata.json` | RDF ↔ carrier | shared model; profile loss is located |
| `datacite-4.6` | `datacite.xml` | RDF ↔ carrier | shared model; profile loss is located |
| `dcat-3` | `dcat.jsonld` | RDF ↔ carrier | shared model; profile loss is located |
| `frictionless-data-package-1` | `datapackage.json` | RDF ↔ carrier | shared model; profile loss is located |

The type distinction between `ProjectionProfile` and `LiftProfile` matters:
curated CSVW terms, OBO Graphs, and SKOS cannot even be named as lift profiles.
They are useful views, not pretend interchange formats.

## One canonical LPG model

The four labeled-property-graph syntaxes are adapters over one typed LPG model,
not independent RDF mappings. Nodes, edges, labels, typed property atoms, graph
context, reifiers, annotations, and exact RDF statements are ordered and
validated once. This gives all four carriers the same identity and reverse
mapping.

Property graphs do not define RDF semantics. PurRDF therefore records each
semantic lowering in the RDF-to-LPG ledger even though the canonical package
also retains exact RDF sideband for reconstruction. A carrier consumer may use
the native LPG view; the sideband remains the authority for an RDF lift.

Readers accept the complete form PurRDF emits and reject drift: wrong headers,
duplicate rows or keys, dangling endpoints, token-map inconsistencies, unknown
Cypher statements, unsafe XML, unexpected package members, and non-canonical
encodings all fail.

### LPG scope, limits, progress, and memory

Every LPG configuration contains a mandatory `scope`. `{"mode":"all"}` is the
only way to request every graph and predicate; omission never means “all.” A
selective scope independently controls the default graph, exact named-graph
terms, predicate allow/deny sets, node RDF types, and native edge predicates.
Named blank graphs retain their scope ordinal, and every selector IRI must be
absolute.

Selection is one closed RDF 1.2 operation. Node types are indexed from
graph-selected `rdf_type` statements even when that predicate is omitted from
the output. A retained edge retains its endpoints. Reifiers and annotations are
retained only when their source statement survives, and annotation predicates
obey the same predicate selector. These rules prevent a selected property graph
from containing dangling sideband.

`LpgExecutionLimits` independently bounds input records scanned, model records,
nodes, and edges. `ProjectionLimits` bounds artifact count, one-artifact bytes,
total artifact-body bytes, canonical USTAR bytes, and recursive RDF-term depth.
Each bound is consumed before the corresponding model mutation or sink write;
the first excess is a typed hard error. The engine does not paginate. Splitting
one canonically ordered, exactly reversible carrier would make page identity and
cross-page endpoints order-sensitive; callers instead choose a narrower scope
or a larger explicit bound.

The direct sink path retains the selected canonical LPG model because stable
backend-independent ordering, type selection, endpoint closure, and exact RDF
sideband require that bounded sort/index. It then emits each artifact in lexical
path order in chunks no larger than 16 KiB, retaining neither complete artifact
bodies nor a USTAR buffer. The archive convenience path additionally retains
materialized artifacts and the final archive. Thus the sink path is bounded by
the selected model plus encoder scratch and one chunk; it is not a constant-memory
RDF-to-LPG mapper.

Progress observers receive monotonic `scanning`, `building`, `writing`,
`complete`, or `aborted` snapshots with input/model/node/edge, finished-artifact,
body-byte, and active-path counters. A sink or observer failure aborts the active
transaction. A sink must stage partial state, publish only on `commit_package`,
and discard it on `abort_package`.

## CSVW

The lower-level CSVW API models annotated table groups, schemas, dialects,
columns, rows, inherited properties, datatypes and formats, language and text
direction, null/default/separator handling, virtual or suppressed columns,
primary and foreign keys, row titles, annotations, and URI templates. It
supports the standard CSVW CSV-to-RDF processing modes over a complete in-memory
package; filesystem and network discovery remain caller responsibilities.

The `csvw-exact` archive profile uses that machinery to carry RDF 1.2 without
loss. Its canonical tables preserve terms, quads, named graph placement,
recursive triple terms, reifier bindings, annotations, datatypes, language,
direction, and blank-node scope. A valid exact round trip has an empty ledger.

The separate `csvw-terms` profile is a deliberately curated wide-table view.
It is for compact catalogs such as classes, properties, individuals, business
entities, or release inventories. Those names have no built-in meaning: every
table, selector, predicate, datatype, and identity rule is supplied by the
caller. The profile never imports an ontology model or assumes RDF, RDFS, OWL,
or any application vocabulary.

A `CsvwTermsConfig` contains only mandatory policy:

- `csvw` is the complete normative CSVW context, vocabulary, processing mode,
  record bound, and package limits;
- `metadata_path` is the safe package-relative metadata member;
- `graph_selection` is either explicit `all` or an exact default-graph flag and
  set of named-graph IRIs;
- each ordered table declaration supplies a stable name, absolute table URL,
  artifact path, row selector, visible subject-IRI column, and one or more
  ordered predicate columns;
- a selector may constrain any/all/none RDF types through an explicitly named
  type predicate and may constrain subject IRI prefixes; the type predicate is
  present exactly when a type set is non-empty, while an empty prefix set means
  no subject-namespace constraint;
- a predicate column accepts either exact IRI objects or literals with exact
  datatype, language, and RDF 1.2 direction facets, plus requiredness and an
  explicit one-or-many cardinality;
- `execution_limits` bounds total output rows, represented values, and values
  in one cell independently of the package and input-record bounds.

Table overlap is intentional. A subject may appear in several views when it
matches several selectors. Duplicate table identities, paths, column names, or
mapped predicates are rejected. `One` fails on a second matching value.
`Many` sorts RDF terms canonically and joins them with the caller's separator;
an actual value containing that separator fails instead of creating an
ambiguous cell. RDF direct-value statements do not carry a source sequence, so
the profile does not fabricate CSVW ordered-list semantics.

Rows are ordered by subject IRI, values by canonical RDF term order, columns and
tables by their declaration order, and archive members by lexical path. The
same dataset and configuration therefore produce the same CSV, metadata, and
USTAR bytes regardless of source interning or statement insertion order.

Every source row is accounted for. Unselected graphs or subjects, blank or
triple-term subjects, unmapped predicates, facet-mismatched objects, selected
named-graph placement, empty named graphs, reifier bindings, and annotations
produce stable source-located ledger entries. The generated tables themselves
can be read with the normative `read_csvw` API, but they cannot reconstruct
omitted source RDF. RDF 1.2 direction remains exact in the annotated CSVW value
and `textDirection` metadata; the W3C CSVW-to-RDF algorithm itself targets RDF
1.1 and therefore returns the language literal without direction. `csvw-terms`
consequently has no `LiftProfile` variant and contains no hidden exact sideband;
use `csvw-exact` whenever archival fidelity or an RDF round trip is required.

## Research-object carriers

Croissant 1.1, RO-Crate 1.3, DataCite Metadata Schema 4.6, DCAT 3, and
Frictionless Data Package v1 are adapters over one typed `ResearchObjectModel`.
The model covers dataset identity and description, identifiers and dates,
agents, licenses, resources and checksums, activities, record sets, and fields.
It is the N-to-N semantic pivot: a document can be lifted to caller-vocabulary
RDF and projected into any other profile without a format-pair implementation.

The three JSON-LD profiles are completely offline. Croissant, RO-Crate, and
DCAT configuration supplies both the exact accepted/emitted `@context` JSON and
the complete term-to-absolute-IRI definition map. PurRDF never dereferences a
context URL, follows `@import`, or supplies a vocabulary. Expanded graphs are
validated through the same native RDF 1.2 JSON-LD engine used elsewhere.

DataCite configuration supplies the namespace, schema location,
XML-Schema-instance IRI, controlled values, and common RDF roles. Its reader is
namespace-aware and rejects DTD/entity input. A separate identifier is used when
present; otherwise the mandatory caller/document dataset identity is the
primary identifier, so no DOI is synthesized. Frictionless configuration
supplies the exact package profile and package name. A resource without a
separate locator uses its caller-bounded relative entity identity as the safe
Data Package path; no new IRI is created.

Each native reader accepts the complete profile form emitted by PurRDF and
rejects duplicate members, dangling references, unsafe relative paths,
incorrect context/profile identity, ambiguous cardinality, and resource-limit
excesses. Format-specific constructs outside the shared model are represented
by stable, location-bearing ledger entries. The committed adversarial fixtures
exercise a non-empty reverse ledger for every profile; a 5×5 metamorphic matrix
proves the shared semantic intersection stabilizes through every source/target
pair.

## OBO Graphs and SKOS views

The OBO Graphs writer emits version 0.3.2 nodes, edges and metadata plus directly
representable equivalent-node sets, logical definitions, restrictions,
domain/range axioms, and property chains. The caller supplies the graph identity
and every RDF/RDFS/OWL/OBO role. Output is checked against the pinned official
0.3.2 JSON Schema.

The SKOS writer maps a caller-selected RDF graph into a caller-identified concept
scheme. It supports concepts, labels, notation, documentation, hierarchy,
mapping relations, membership, and top concepts, while enforcing the relevant
SKOS integrity conditions. Target SKOS role IRIs are supplied just like source
roles; PurRDF is a carrier and does not mint even standard vocabulary defaults.

Both views record every omitted or widened source construct, named-graph
placement, and RDF 1.2 statement-layer row with a stable source location.

## Configuration

The production archive API accepts strict tagged JSON of the form
`{"profile":"…","config":{…}}`. Unknown fields and a profile/config mismatch
fail. There is no default configuration. A minimal generic LPG example is:

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

To retain one named graph while admitting every predicate and type, replace the
scope object with:

```json
{
  "mode": "select",
  "include_default_graph": false,
  "named_graphs": {
    "mode": "only",
    "include": [{"kind": "iri", "value": "https://example.org/business"}],
    "exclude": []
  },
  "predicates": {"mode": "all", "deny": []},
  "node_types": {"mode": "all", "deny": []},
  "edge_types": {"mode": "all", "deny": []}
}
```

The package and canonical-model bounds apply on write and read as relevant; the
input-record bound governs RDF projection scans. Together they cover member
count, one-member bytes, total body bytes, encoded archive bytes, input/model
records, nodes, edges, and recursive RDF term depth. They are trust-boundary
policy and must be chosen by the application.

Research-object configurations add a mandatory `common` object containing the
complete `roles`, `identity`, and bounded `policy` maps. Profile-specific
vocabulary roles, context data, schema identity, controlled values, and native
profile identity are also mandatory. Complete runnable `example.org`
configurations for all five profiles are under
`crates/rdf/tests/fixtures/research-objects/carrier/`; they are examples, never
library defaults.

The complete strict tagged-JSON shape for curated CSVW is exercised by
`crates/rdf/tests/fixtures/csvw-terms.json`. Its graph selector is explicit:

```json
{
  "kind": "include",
  "default_graph": true,
  "named_graphs": ["https://example.org/business"]
}
```

Each table selector then names its own caller vocabulary and row population:

```json
{
  "type_predicate": "https://example.org/type",
  "any_types": ["https://example.org/Class"],
  "all_types": [],
  "none_types": ["https://example.org/Retired"],
  "iri_prefixes": ["https://example.org/vocab/"]
}
```

The runnable `csvw_terms` Rust example constructs the complete configuration
with three ordinary table declarations—classes, properties, and individuals—
and writes the canonical archive:

```sh
cargo run -p purrdf-rdf --example csvw_terms -- /tmp/terms.tar
```

## Rust archive API

```rust,ignore
use purrdf::{
    LiftProfile, LpgConfig, LpgExecutionLimits, LpgScope, ProjectionConfig,
    ProjectionLimits, ProjectionProfile, lift_archive, parse_dataset,
    project_archive,
};

let dataset = parse_dataset(
    b"<https://example.org/alice> <https://example.org/knows> <https://example.org/bob> .",
    "text/turtle",
    None,
)?;
let limits = ProjectionLimits::new(16, 1_000_000, 4_000_000, 5_000_000, 16)?;
let config = ProjectionConfig::LpgCsv(LpgConfig::new(
    "https://example.org/type",
    LpgScope::all(),
    limits,
    LpgExecutionLimits::new(1_000, 1_000, 1_000, 1_000)?,
)?);

let package = project_archive(dataset.as_ref(), ProjectionProfile::LpgCsv, &config)?;
let lifted = lift_archive(&package.archive, LiftProfile::LpgCsv, &config)?;
assert_eq!(lifted.dataset.quad_count(), 1);
```

`ProjectionArchive` contains the profile, USTAR bytes, and loss ledger.
`ProjectionLift` contains the reconstructed immutable dataset and its lift
ledger. `project_lpg_artifacts_to_sink` dispatches the four LPG profiles into a
caller-owned `ProjectionArtifactSink` through the same configuration and mapping
engine; `LpgProgressObserver` supplies structured progress. Lower-level APIs
also expose the typed LPG, CSVW, OBO, SKOS, and in-memory research-object/artifact
models.

## Other production surfaces

The surface names follow the same profile/config/archive contract:

| Host | Materialized project | Direct LPG artifacts | Lift |
| --- | --- | --- | --- |
| Rust | `project_archive` | `project_lpg_artifacts_to_sink` | `lift_archive` |
| CLI | `purrdf project` | — | `purrdf lift` |
| Python | `purrdf.project(...)` | `purrdf.project_artifacts(...)` | `purrdf.lift(...)` |
| JavaScript | `dataset.project(...)` | — | `liftProjection(...)` |
| C | `purrdf_project(...)` | — | `purrdf_lift(...)` |

Runnable examples live at:

- `crates/rdf/examples/projection_archive.rs`
- `crates/rdf/examples/csvw_terms.rs`
- `crates/rdf/examples/research_object_roundtrip.rs`
- `crates/cli/examples/projection-roundtrip.sh`
- `bindings/python/examples/projection_roundtrip.py`
- `bindings/python/examples/projection_stream.py`
- `crates/rdf-wasm/js/examples/projection-roundtrip.mjs`
- `crates/rdf-capi/examples/projection_roundtrip.c`

## Determinism and verification

Archive members use safe POSIX-relative paths in lexical order. USTAR headers,
metadata, checksums, padding, and trailer are fixed. A reader validates the
archive and requires its canonical re-encoding to match the input bytes, so it
does not silently normalize attacker-controlled alternatives.

The pinned W3C CSVW manifests exercise 270 RDF cases and 282 validation cases.
A locked independent `csvw` implementation validates production output and
rejects deliberate metadata/data corruption. The OBO writer is independently
validated against the pinned official schema with corruption probes. Run the
whole projection verification slice with:

```sh
make projection-oracles
cargo bench -p purrdf-rdf --bench projections -- --quick
```

The benchmark is report-only. It measures RDF-to-LPG mapping, scoped versus
explicit-all mapping over a 20-graph carrier, materialized package versus direct
sink output for every LPG syntax, every LPG read path, exact CSVW write/read,
exact-versus-curated CSVW over the same 12,000-quad carrier, one-graph versus
all-graph curated scope, OBO Graphs and SKOS projection, the shared
research-object model, and all five research-object write/read paths. It also
reports allocation and artifact-body-size observations over deterministic
fixtures.
