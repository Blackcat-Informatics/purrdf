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
- The deterministic `$defs` transliteration in
  `crates/pipeline/src/stages/pydantic.rs` at
  `6cfd86d0ac9450e8cfdc1ae0c54acfea326b186e` was used as the migration reference
  for `purrdf-shapes::pydantic`. PurRDF removes its repository, ontology,
  namespace, slice-routing, and fixed-package coupling; the carrier API consumes
  `CompiledSchema` in memory, takes package prose from the caller, and records
  runtime projection gaps on the shared closed loss ledger.
- The legacy LinkML YAML model in
  `crates/pipeline/src/stages/schemas.rs` at
  `c91195e0c300cad9c9a32c8580c2910a6fd48fc1` was used solely as migration
  evidence for behavior that PurRDF must subsume and replace. Its private
  OWL/FoldView structures, fixed identity, shallow range mapping, and coupled
  TypeScript/GraphQL model are not reused architecture. The replacement
  `purrdf-shapes::linkml` API consumes `CompiledSchema`, requires all identity
  and vocabulary from the caller, preserves a canonical LinkML 1.11 document,
  and records projection gaps through the closed loss ledger. The legacy model
  is intended for deletion once this replacement is integrated, not
  preservation as a downstream contract.
- The legacy `render_typescript` path in
  `crates/pipeline/src/stages/schemas.rs` at
  `c91195e0c300cad9c9a32c8580c2910a6fd48fc1` was used only as evidence of the
  consumer artifact to replace. Its LinkML-coupled private model, normalized
  property identifiers, all-optional fields, local-name runtime enums, scalar
  fallbacks, and fixed downstream identity are deliberately discarded. The
  replacement `purrdf-shapes::typescript` projection consumes
  `CompiledSchema`, preserves exact JSON property names and requiredness,
  requires caller-owned package identity and prose, exposes a reversible type
  map, and locates every non-projectable assertion on a closed loss ledger. The
  old renderer and its shared private schema model are intended for deletion
  once this replacement is integrated; no downstream type contract is being
  preserved.
- The legacy `render_graphql` path in
  `crates/pipeline/src/stages/schemas.rs` at
  `c91195e0c300cad9c9a32c8580c2910a6fd48fc1` was likewise used only as
  migration evidence for a consumer artifact that PurRDF must subsume and
  replace. Its output-only, LinkML-coupled private model, fabricated `id`/`iri`
  fields, normalized names without a reverse map, all-nullable fields, broad
  scalar collapse, and fixed GMEOW identity are deliberately discarded. The
  replacement `purrdf-shapes::graphql` projection consumes `CompiledSchema`,
  requires caller-owned identity, prose, and fallback-scalar name, emits paired
  output/input GraphQL September 2025 SDL, retains a canonical reversible name
  map and value codec, and locates every coercion difference on a closed loss
  ledger verified against GraphQL.js. The old renderer and shared legacy model
  are intended for deletion when gmeow integrates this replacement; that
  consumer cutover is not yet complete and no downstream type contract is being
  preserved.
- The legacy graph/tabular writers in `crates/pipeline/src/stages/lpg.rs` and
  `crates/pipeline/src/stages/export.rs` at
  `d7745068f59b6dee187ab6b806bd2c04c9a1280a` were used solely as migration
  evidence for outputs that PurRDF must subsume and replace. Their private
  carrier structs, hardcoded GMEOW graph/vocabulary identity, local-name and
  prefix shortening, fixed filenames, coupled pipeline context, writer-only
  behavior, and ad hoc layouts are deliberately discarded rather than retained
  as reusable models. The replacement `purrdf-rdf::projections` surface uses one
  caller-configured canonical LPG model with four strict adapters, a standards
  CSVW engine plus exact RDF 1.2 profile, and typed OBO Graphs 0.3.2 and SKOS
  views. It requires caller-owned identity, vocabulary, limits, and policy;
  produces deterministic bounded archives; and computes closed located loss
  ledgers on every path. The legacy types and writers are intended for deletion
  when gmeow integrates these replacements. That consumer cutover is not yet
  complete, and no downstream type or byte-layout contract is being preserved.
- The legacy research-object stage in
  `crates/pipeline/src/stages/research_objects.rs` at
  `154921ddce1797b220877598f75d838e2075dc42` was used solely as migration
  evidence for value correspondences that PurRDF must subsume and replace. Its
  worked-example store/model, fixed GMEOW vocabulary and identities,
  placeholder DOI generation, filesystem-bound graph loading, declared-loss
  strings, Python/rdflib/ElementTree byte-parity targets, and writer-only
  Croissant/RO-Crate/DataCite/DCAT/Frictionless outputs are deliberately
  discarded. The replacement `purrdf-rdf::projections::research_object` surface
  uses one typed caller-vocabulary semantic pivot, five strict bidirectional
  versioned codecs, offline JSON-LD interpretation, deterministic bounded USTAR
  carriers, and closed located runtime loss ledgers. The legacy types and stage
  are intended for deletion when gmeow integrates the replacement; that
  consumer cutover is not yet complete, and no legacy type or byte-layout
  contract is preserved.

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
- The downstream cutover is still in progress. Legacy consumer models and
  renderers are migration evidence to delete as their PurRDF replacements are
  integrated, not compatibility surfaces to preserve.
- See `docs/CUTOVER.md` for the publish order, local gates, and dependency
  replacement rules.
