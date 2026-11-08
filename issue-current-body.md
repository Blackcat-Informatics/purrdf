Prepare the coherent PurRDF 4.0.1 release from qualified main. Publish
`purrdf-testkit`, `purrdf-alloc-probe` and `purrdf-conformance-kit` as supported
Rust development libraries. Reconcile package metadata, versioned dependency
edges and topological publish order, self-contained packaging, portable library
builds, recipient licenses, release inventories and consumer documentation.
Preserve normal commit and publication verification.

All three empty `0.0.0` registry bootstrap versions are published and confirmed.
The maintainer confirmed Trusted Publisher setup; strict public preflight
verified publishing enforcement on all forty release records. Functional
versions and release tags remain unpublished.

Release preparation is in #1237. The separately owned updated EARL suite and
harness are in #1238; land that qualified change before final release
qualification. Coordinate its evidence and tagged report regeneration without
competing source edits or modifying published 4.0.0 assets.

Complete coherent Rust/Python/npm versions, the generated C header and reviewed
release notes in the dedicated worktree. Finish all existing release gates on
clean synchronized main, then use the coherent atomic tag flow and verify all
three trusted publication lanes. Keep this issue open through actual release
publication. Current sweep integrations remain isolated; 603 and the remaining
1220 depth work do not block this patch release.
