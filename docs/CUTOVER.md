# PurRDF Cutover

This is the cutover checklist for replacing the in-tree RDF/GTS carrier stack in
`gmeow-ontology` with this repository.

## Current Boundary

- `purrdf-core` owns the RDF 1.2 primitive model, frozen dataset IR,
  diagnostics, provenance, loss ledger, and store traits.
- `purrdf-rdf` owns RDF text/XML/JSON-LD adapters and the first-class GTS
  adapter surface over purrdf primitives.
- `purrdf` is the user-facing umbrella crate: it re-exports `purrdf-rdf` and
  includes the first-class slice and shape APIs.
- `purrdf-gts` owns the GTS container only: CBOR sequence, terms/quads, fold,
  verification, signing/encryption metadata, blobs, files, and transport policy.
- `purrdf-slice` owns the slice/catalog carrier IR, dataset-level wrappers,
  ownership/dependency analysis, and projection inputs.
- `purrdf-shapes` owns SHACL validation.
- ShEx remains a cutover follow-up. The current `gmeow-ontology` checkout has
  ShEx as export/projection logic and metadata, not a standalone crate.

Do not copy the ontology corpus into purrdf. The ontology sources, generated
artifacts, and pipeline orchestration stay in `gmeow-ontology`; purrdf is the
library/toolkit layer they consume.

## Local Cutover Branch

The prepared ontology worktree is:

```sh
/home/paudley/Active/gmeow-ontology/.worktrees/purrdf-cutover
```

It is on branch:

```sh
paudley/purrdf-cutover
```

Until the purrdf crates are published, test the ontology cutover with explicit
path dependencies pointing at `/home/paudley/Active/purrdf/crates/*`. After
publishing, replace those path dependencies with `0.1.1` registry dependencies.

## Publish Order

Publish the dependency leaves first:

1. `purrdf-events`
2. `purrdf-iri`
3. `purrdf-xsd`
4. `purrdf-gts`
5. `purrdf-core`
6. `purrdf-sparql-algebra`
7. `purrdf-sparql-results`
8. `purrdf-sparql-eval`
9. `purrdf-rdf`
10. `purrdf-slice`
11. `purrdf-shapes`
12. `purrdf`
13. `purrdf-wasm`

Every internal path dependency in this workspace carries `version = "0.1.1"` so
`cargo package` can resolve the same graph after publication.

The GitHub release workflow is `.github/workflows/release-cargo.yaml`; it gates
the release set with `cargo check --target wasm32-unknown-unknown --lib` before
packaging. See `docs/RELEASE.md` for the Trusted Publisher entries and
attestation checks.

`purrdf-python` is the PyPI extension package under `bindings/python`,
`purrdf-sparql-conformance` stays internal for the W3C fixture gate, and
`purrdf-capi` stays out of the core crates.io lane because it is a native C ABI
artifact rather than a wasm-capable crate.

## PurRDF Gates

Run these before using the cutover worktree:

```sh
cargo metadata --no-deps
cargo fmt --all --check
cargo check --workspace --lib --tests
cargo check --target wasm32-unknown-unknown --lib \
  -p purrdf-events -p purrdf-iri -p purrdf-xsd -p purrdf-gts \
  -p purrdf-core -p purrdf-sparql-algebra -p purrdf-sparql-results \
  -p purrdf-sparql-eval -p purrdf-rdf -p purrdf-slice \
  -p purrdf-shapes -p purrdf -p purrdf-wasm
cargo test -p purrdf-gts --test transport
cargo test -p purrdf-slice
```

`cargo test --workspace` is the broader gate; it may include long conformance
lanes depending on the local profile and fixture availability.

## Ontology Cutover Shape

In `gmeow-ontology/.worktrees/purrdf-cutover`:

1. Replace in-tree `gmeow-rdf`/RDF-stack dependencies with purrdf crates.
2. Replace `gmeow-gts` RDF/native-codec assumptions with `purrdf` APIs; use
   `purrdf-gts` only for container transport.
3. Keep slice/corpus ownership in `gmeow-ontology`; consume `purrdf-slice` for
   catalog and dependency-analysis primitives.
4. Keep SHACL calls on `purrdf-shapes`.
5. Leave ShEx projection logic in the ontology pipeline until the purrdf ShEx API
   is intentionally designed.
6. Regenerate ontology artifacts from canonical sources, then run the repo gate.
