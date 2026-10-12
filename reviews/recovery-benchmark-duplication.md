# Duplicate geo benchmark review

Read-only source finding, recorded 2026-10-11. No builds, timing runs, process
changes or shipping-source edits were performed. This record persists the
completed bounded review; it introduces no additional acceptance requirement.

## Finding and necessity

The two kernel benchmark copies have no distinct qualification purpose:

- `crates/geo/benches/relate.rs` and
  `crates/geo-kernel/benches/relate.rs` differ only in their `purrdf_geo` versus
  `purrdf_geo_kernel` import (line 37).
- `crates/geo/benches/geojson_parse.rs` and
  `crates/geo-kernel/benches/geojson_parse.rs` differ only in the corresponding
  import and a rustdoc crate name (lines 17 and 27).

`crates/geo/src/lib.rs:12` directly re-exports the kernel. Both copies therefore
measure the same types, functions, fixtures and workloads; the facade adds no
computational body. Neither the accepted combined plan nor the inspected
qualification requirements requires duplicate timing of these aliases. No
helper-ledger sanction for these duplicated bodies was found.

This is an existing hygiene defect, not optional unrelated cleanup:
`scripts/check-shared-helpers.py:8–29` explicitly covers benchmark/support code
and refuses unsanctioned isomorphic bodies. `crates/helper-census/src/main.rs:55`
documents the same check. The gate was inspected, not executed by this review;
no actual gate failure is claimed here.

## Bounded repair

Retain the existing public facade benchmarks and all four distinct new kernel
benchmarks (`cells`, `covers`, `geodesic`, `transforms`). Remove only:

1. The two duplicate kernel benchmark files.
2. Their `[[bench]]` registrations in `crates/geo-kernel/Cargo.toml`.
3. Their two benchmark mapping rows in `docs/design/purrdf-simd.md:889–890`
   (line numbers as reviewed).

Do not add a ledger exemption or a cross-crate source include to preserve the
duplicates. No assembly site, threshold, measured cell, build-package entry or
function-row attribution needs changing for this removal. The retained facade
benchmarks still reach `geo.relate`, `geo.exact-limbs` and
`geo.json-write-string` at their same computational homes. The kernel retains
its measured function rows and its four distinct benchmark targets.

## Assembly and qualification applicability

Leave the active ASM run untouched until it finishes. The ASM compiler commands
use `--lib` (`scripts/check-simd-asm.py:1338–1348`), so the duplicate benchmark
targets contribute no additional assembly qualification. Coverage validation
does require bench-map rows to correspond to actual files
(`check-simd-asm.py:1707–1718`), hence the two matching row deletions above.

The report identity includes every tracked and untracked unignored file's
content (`scripts/simd_asm_runtime.py:52–65`), independently of whether Cargo
compiles it. Removing these files and changing the target registrations/doc
rows therefore changes report identity. `report_identity` at line 289 captures
that identity, and `merge_reports` at lines 297–306 rejects stale reports.
Do not present predecessor reports as final-source shard reports.

After the bounded repair, refresh the existing final ASM gate through its
normal persistent Cargo/assembly caches. No forced `--fresh`, relaxed gate or
new acceptance ceremony is warranted. Applicable measurements of unchanged
computational source remain useful evidence, but final report identity must be
truthful. Deleting unused duplicate benchmark targets does not justify
repeating completed geo correctness corpora or performance measurements.
