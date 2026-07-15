# Provenance

This repository is a copy-and-rename extraction assembled on the
`purrdf-extraction` branch.

Source snapshots:

- `../gmeow-ontology` at `2e613ac36c1ba896d7593585424e62d64d2a560a`
- `../gmeow-gts` at `a7949a474a5670a25fdde8f0b76146b1fa0f458c`

Copied from `gmeow-ontology`:

- RDF kernel crates: `rdf`, `rdf-core`, `rdf-events`, `rdf-capi`, `rdf-wasm`
- IRI and XSD support crates: `iri`, `xsd`
- SPARQL crates: `sparql-algebra`, `sparql-eval`, `sparql-results`,
  `sparql-conformance`
- SHACL/shape validation: `shacl` copied as `shapes`
- Carrier IR and dataset/slice wrappers: `slice`
- Python package sources under `python/src/purrdf`
- The normalized five-table Parquet projection in
  `crates/pipeline/src/stages/parquet.rs` was used as the migration reference for
  `purrdf-columnar`; PurRDF replaces its Arrow/Snappy writer-only path with the
  first-party bidirectional RDF 1.2-complete codec documented in
  `docs/COLUMNAR.md`.

Copied from `gmeow-gts`:

- Rust GTS transport engine copied as `crates/gts`
- GTS conformance vectors under `vectors`
- GTS specification and implementer docs under `docs`
- SVG brand assets under `docs`

Extraction policy:

- Source repositories are read-only during this phase.
- `purrdf-core` is the transport-independent primitive layer.
- `purrdf-slice` carries the ontology-structure layer: slice catalogs,
  dataset-level wrappers, ownership/dependency analysis, and generated
  projection inputs.
- `purrdf-gts` is the GTS container engine: CBOR sequence, transforms, fold,
  verification, signing, encryption, files, and transport policy.
- RDF text/native-store/profile codecs formerly exposed by `gmeow-gts` are not
  exported by `purrdf-gts`; purrdf owns those adapters on top of purrdf
  primitives.
- SHACL and ShEx are part of purrdf's shape scope. SHACL is present in
  `crates/shapes`; current source checkouts expose ShEx as projection/export
  logic and dependency metadata rather than a standalone crate, so a purrdf ShEx
  API still needs to be defined.

Cutover staging:

- `../gmeow-ontology/.worktrees/purrdf-cutover` exists on branch
  `paudley/purrdf-cutover`.
- See `docs/CUTOVER.md` for the publish order, local gates, and dependency
  replacement rules.
