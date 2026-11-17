# Issue #1351: Switch canonical output to the new labelling engine for 5.0

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

Switch PurRDF's canonical output to the new labelling engine (specification `nested-refinement-labelling`, version 1, revision 3.11, frozen, sha256 `a2dc6ea69735f62cde18be17bc14890d89b4416e8dcf3bba42e3cfe55e283c15`), as a breaking change in the 5.0 release.

## Decisions (recorded, reversible in scope)

- RDFC-1.0 stays a named public entry point; the W3C canonicalization suite keeps passing unchanged.
- The default canonicalization family routes to the new engine.
- The v4 algorithm (the `purrdf-rdfc12` overlay and its flat form) stays callable as a read-side recompute. No writer records a v4 digest in a new artifact.
- PACK format version 1 means a v4 digest; the 5.0 writer emits format version 2 with the new digest. Other containers (GTS-certify, columnar, embedding) are checked per container before any consumer statement.
- The new profile carries the specification's identifier (`nested-refinement-labelling`, version 1). New constants; `CANON_PROFILE_*` names the legacy profile.
- The host bound argument maps onto the engine's typed limit; refusal is the typed `limit` outcome.
- The canonical document is the canonical line sequence of section 7.6, spelled as N-Quads 1.2 with decimal labels.
- `DatasetStateDigest` keeps its own algorithm and vectors.
- Frozen vectors under `vectors/` are not regenerated; a new first-party corpus gets a new directory and digest root.

## Scope

- Public canonicalization entry points in Rust, Python, wasm, C and the CLI route to the new engine; the RDFC-1.0 entry point is unchanged.
- Constants, versions and digest roots; the PACK, GTS, columnar and embedding writers record the new digest under the new version.
- Goldens and pins that move: each changed value is named and justified in the commit that moves it.
- Migration guide section with the 5.0 line sequence precise enough to reimplement, the flat presentation version change, and the read-side v4 recompute.
- CHANGELOG entry; removal of the versioning-history text in the canonical profile document; stale "v2" text corrected.

## Acceptance

- The W3C RDFC-1.0 suite passes unchanged.
- The new engine's output matches the 15 worked-example blocks of the frozen text.
- The v4 recompute reproduces stored v4 digests for the PACK, GTS and columnar fixtures.
- Each host entry point has a refusal case and a valid neighbouring case.
- No frozen vector is regenerated.

## Out of scope

- The engine's optimisation routes (tracked on the engine's own issue).
- Consumer migrations.


## Comments (0)

