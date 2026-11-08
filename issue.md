# Issue #1235: Release 4.0.1 and publish shared test support crates

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Prepare the coherent PurRDF 4.0.1 release from qualified main. Publish purrdf-testkit and purrdf-alloc-probe as supported Rust library crates, including their empty 0.0.0 registry bootstrap records so the maintainer can configure trusted publishing. Reconcile package metadata, dependency versions and topological publish order, packaging, portable builds, release inventories and consumer documentation. Preserve normal commit and publication verification.

Updated EARL reports are owned by another active session. Consume its qualified changes and final release artifacts without duplicating that work or touching its checkout. Complete version coherence, generated C header and reviewed release notes in a new dedicated worktree. Functional release tags and publication require all existing release gates and configured trusted publishers. Keep current sweep integrations isolated and coordinate ready merges through ghprsq.


## Comments (0)

