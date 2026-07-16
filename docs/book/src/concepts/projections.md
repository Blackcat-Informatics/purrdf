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
| `obo-graphs` | OBO Graphs 0.3.2 JSON | RDF → view | located, closed loss ledger |
| `skos` | SKOS Turtle | RDF → view | located, closed loss ledger |
| `croissant-1.1` | `croissant.json` | RDF ↔ carrier | shared model; profile loss is located |
| `ro-crate-1.3` | `ro-crate-metadata.json` | RDF ↔ carrier | shared model; profile loss is located |
| `datacite-4.6` | `datacite.xml` | RDF ↔ carrier | shared model; profile loss is located |
| `dcat-3` | `dcat.jsonld` | RDF ↔ carrier | shared model; profile loss is located |
| `frictionless-data-package-1` | `datapackage.json` | RDF ↔ carrier | shared model; profile loss is located |

The type distinction between `ProjectionProfile` and `LiftProfile` matters:
OBO Graphs and SKOS cannot even be named as lift profiles. They are useful views,
not pretend interchange formats.

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
    "limits": {
      "max_artifacts": 16,
      "max_artifact_bytes": 1000000,
      "max_total_bytes": 4000000,
      "max_archive_bytes": 5000000,
      "max_term_depth": 16
    },
    "max_records": 1000
  }
}
```

The bounds apply on both write and read. They cover member count, one-member
bytes, total body bytes, encoded archive bytes, records, and recursive RDF term
depth. They are trust-boundary policy and must be chosen by the application.

Research-object configurations add a mandatory `common` object containing the
complete `roles`, `identity`, and bounded `policy` maps. Profile-specific
vocabulary roles, context data, schema identity, controlled values, and native
profile identity are also mandatory. Complete runnable `example.org`
configurations for all five profiles are under
`crates/rdf/tests/fixtures/research-objects/carrier/`; they are examples, never
library defaults.

## Rust archive API

```rust,ignore
use purrdf::{
    LiftProfile, LpgConfig, ProjectionConfig, ProjectionLimits,
    ProjectionProfile, lift_archive, parse_dataset, project_archive,
};

let dataset = parse_dataset(
    b"<https://example.org/alice> <https://example.org/knows> <https://example.org/bob> .",
    "text/turtle",
    None,
)?;
let limits = ProjectionLimits::new(16, 1_000_000, 4_000_000, 5_000_000, 16)?;
let config = ProjectionConfig::LpgCsv(LpgConfig::new(
    "https://example.org/type",
    limits,
    1_000,
)?);

let package = project_archive(dataset.as_ref(), ProjectionProfile::LpgCsv, &config)?;
let lifted = lift_archive(&package.archive, LiftProfile::LpgCsv, &config)?;
assert_eq!(lifted.dataset.quad_count(), 1);
```

`ProjectionArchive` contains the profile, USTAR bytes, and loss ledger.
`ProjectionLift` contains the reconstructed immutable dataset and its lift
ledger. Lower-level APIs expose the typed LPG, CSVW, OBO, SKOS, and in-memory
research-object/artifact models when an application needs to inspect or
materialize individual members instead of the unified archive.

## Other production surfaces

The surface names follow the same profile/config/archive contract:

| Host | Project | Lift |
| --- | --- | --- |
| CLI | `purrdf project` | `purrdf lift` |
| Python | `purrdf.project(...)` | `purrdf.lift(...)` |
| JavaScript | `dataset.project(...)` | `liftProjection(...)` |
| C | `purrdf_project(...)` | `purrdf_lift(...)` |

Runnable examples live at:

- `crates/rdf/examples/projection_archive.rs`
- `crates/rdf/examples/research_object_roundtrip.rs`
- `crates/cli/examples/projection-roundtrip.sh`
- `bindings/python/examples/projection_roundtrip.py`
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

The benchmark is report-only. It measures RDF-to-LPG mapping, every LPG carrier
write/read path, exact CSVW write/read, OBO Graphs and SKOS projection, the
shared research-object model, and all five research-object write/read paths. It
also reports allocation observations over deterministic fixtures.
