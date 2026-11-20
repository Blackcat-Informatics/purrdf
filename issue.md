# Issue #1399: v5.0: use a collision-resistant identity for GeoIndex source binding

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem

`purrdf_geo_kernel::GeoIndex::source_fingerprint` returns a published, frozen FNV-1a `u64`. `verify_binding` uses equality of those values as its only check that the rebuilt index represents the same source. `IndexBindingError::Source` reports the two 64-bit values. #1345 requires a collision-resistant public source identity.

## Required before 5.0

- Replace the 64-bit source identity with a domain-separated BLAKE3 digest of the deterministic index projection. Keep the projection's scope, full geometry precision and admission. Use the same identity for native verification, the public accessor and typed mismatch evidence.
- Document the replacement and how callers replace a stored v4 value by recomputing from the source. Verify that the same projection still binds, an adjacent changed projection is refused, and input order leaves the new identity unchanged.

This is a change to source binding. Spatial results, indexing algorithms and unrelated row-set accounting remain under their existing issues.


## Comments (2)

### paudley — 2026-11-20T02:10:38Z

## Plan

**Surface that breaks.** `GeoIndex::source_fingerprint` returns `u64` (`geo-kernel/src/index.rs:691`). It becomes a 32-byte BLAKE3 digest, domain-separated, of the deterministic index projection. The same identity is used by native verification, the public accessor and the typed mismatch evidence. `IndexBindingError::Source` carries 32-byte values instead of two `u64`s. The return type changes, so every caller of the accessor and the error variant changes.

**What changes in stored or exchanged values.**
- The source identity of every index changes. Any stored v4 `u64` no longer matches a recomputed value, even for an unchanged source.
- The index itself, its spatial results and its projection do not change. Only the binding identity changes.
- Open point: I have not confirmed whether `source_fingerprint` is written in any serialized index form. If it is, the serialized format changes too and the migration needs its own note. I will confirm this first, before any code is written.

**What a holder of an existing index does.** A v4 stored value cannot be converted to the new identity, because the 64-bit value does not determine it. The holder rebuilds the index from its source and stores the new 32-byte identity. Binding then verifies against the rebuilt identity. The migration note states this.

**Verification.** The same projection still binds, with the new identity computed from it. An adjacent changed projection is refused. Input order leaves the identity unchanged. Verify the digest is domain-separated from the row-set identity, so the same bytes do not collide across uses.

**Lane.** Touches `crates/geo-kernel` and `sparql-eval/src/geo/relation.rs`, the caller that uses the binding. It shares `sparql-eval` with #1397 but not the same files.

**Estimated size.** Medium. The identity change is small, but the stored-value question decides the migration and must be settled first.


### paudley — 2026-11-20T02:24:19Z

## Correction to the plan, and the answer to the stored-value question

My plan said a holder of an existing index would need to rebuild because a stored version-4 value could not be converted. That premise is not backed by the code, and the answer is more specific.

**Is the source fingerprint written into any serialized index?** No. Checked on `main` at `d2be07c47`:

- `GeoIndex`, `GeoIndexData` and `GeoIndexConfig` derive only `Clone`, `Debug` and, for the config, `PartialEq`. There is no serde derive and no codec in `geo-kernel`, and `Cargo.toml` has no serialization dependency.
- The field is `source_fingerprint: u64` at `geo-kernel/src/index.rs:389`. The accessor `source_fingerprint()` is at `index.rs:691`.
- `GeographicGeoIndex` builds its own digests and never reads the field.
- No binding (`rdf-capi`, `rdf-wasm`, the Python package, `cli`) exposes `GeoIndex` or the fingerprint. Their geo modules wrap a different index type.

**Where the value is observable:**

1. `IndexBindingError::Source { expected: u64, actual: u64 }` (`geo-kernel/src/error/index_config.rs:141-146`). This is returned by `verify_binding` on a failed check. It is transient evidence, not stored. Its `Display` shows both values in hex (`index_config.rs:185`), and its structured visitor emits them as unsigned numbers (`index_config.rs:154`).
2. The `Debug` output of `GeoIndex` and of `GeographicGeoIndex`, which embeds it.
3. The public accessor `source_fingerprint()`. A caller can store its output in its own database. This is the only route by which a stored value can exist outside the repository, and the repository cannot see or migrate it.

**What the migration therefore needs:**

- No in-repository format changes and no stored value to convert. The index can be opened and queried as before.
- A caller that persisted `source_fingerprint()` itself must recompute the new 32-byte identity from its source, since a 64-bit value does not determine it. The migration note says this.
- `verify_binding` with a stored v4 value: a caller passing an old 64-bit value cannot compile against the new type, so there is no silent acceptance. The compile-time break is the refusal.
- Pinned value: `crates/sparql-eval/src/geo/relation.rs:936-945` (`the_source_fingerprint_is_frozen`) pins `0x21e852458fc234f7`. That golden moves, and the change must update it with a stated reason.
- No geo determinism golden pins the fingerprint, despite the plan's wording.

**Open:** whether the structured visitor is serialized to JSON or a wire value anywhere. I did not trace the error pipelines of the bindings. The Rust surface is unaffected by that.


