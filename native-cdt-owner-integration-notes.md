<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Native CDT owner unit: integration and acceptance proposal

The writer has integrated `native-cdt-owner-draft.patch` into the shipping worktree. These notes describe that concrete unit and the supplemental proposal; they do not certify compilation, tests, or complete #508 acceptance. The writer corrected the original draft's `alloc` imports to the lex crate's actual `std` surface and owns grouped qualification. Do not rerun the historical main-patch assembler against the integrated postimage.

## Actual homes and contracts

* `purrdf_lex::allocation::{Admission, Memory, StorageError}` is the neutral buffer home shared with native regex. `Admission::resize(live_bytes)` runs before growth. `Memory` checks real layouts, retains both old and replacement buffers during a fallible replacement, and releases old bytes after old storage dies. A nonempty supplied buffer must already belong to that `Memory` total. `release_vec` releases the vector buffer only; independently allocated element payloads need their own release. `Memory`'s `Debug` requires no `S: Debug` and prints only live bytes. `release_string` covers discarded normalized regex text. Shipping `crates/lex/src/allocation.rs` is authoritative.
* `purrdf_cdt::memory::{Storage, ReadError}` adds exact-layout-admitted native value/triple box factories. `try_parse_cdt` runs the existing scanner and returns `(CdtValue, retained_requested_bytes)`. Callers keep their callback's original grant with the raw payload. No second scanner or grammar is introduced. The original resident parser APIs remain resident adapters.
* `crates/cdt/src/parse.rs`, `limits.rs`, `render.rs`, and `ops.rs` use that same admission path for scanner frames, decoded strings, leaf boxes, map construction/duplicate diagnostics, extent/writer walks, and total-order comparison work. The source map preserves exact lexical errors and duplicate offsets. Comparator work is released before returning `Ordering`. Total comparison remains the native syntactic ordering; numeric promotion and partial SEP value relations are separate existing expression semantics.
* `purrdf_lex::walk::dismantle_owned` is the existing destruction job generalized to heterogeneous owned continuations. CDT reuses vacated child slots and existing boxes. Deep successful and failed construction drops allocate nothing and do not recurse on the machine stack; public CDT term/value shapes remain unchanged.
* `purrdf_iri::absolute_verdict` runs the actual IRI scanner with quiet diagnostic policy. Its tri-state is `Some(true)` valid absolute, `Some(false)` valid relative, `None` malformed. It prevents temporary owned IRI/error strings in the CDT validation seam while preserving the original diagnostic-producing parser body and messages.
* `crates/sparql-eval/src/composite_value.rs` owns `CompositeValue`. An operational payload is immutable `Shared<CompositePayload>` carrying the original `WorkspaceAllocation`; ordinary cloning shares both. Native retained bytes plus `Shared::<CompositePayload>::allocation_layout()` are admitted before publication. There is no raw extraction. Resident raw CDT cloning remains caller-owned. The original typed admission failure is kept in the adapter and takes precedence over lexical interpretation; only a real lexical error becomes ORDER's existing opaque-literal key.
* The actual `modifier.rs` `project_shallow_admitted` and `shallow_order_admitted` use `CompositeValue::parse` and `total_cmp`. These are bounded ORDER's production seams, not an alternative evaluator. Expression `cdt_fn` construction, partial comparisons, and rendering still need the main writer/numeric expression unit's owned-value integration; this artifact does not claim those raw resident helpers are accounted.

## Supplemental artifacts ready for the writer

`native-cdt-documentation-supplement.patch` is a standard diff against the writer's corrected postimage. It only clarifies Memory method/error/ownership contracts, allocation-free CDT destruction, and the quiet IRI verdict. Read-only `patch --dry-run --batch -p1` passed all five affected homes. It does not duplicate the writer's import corrections.

`native-cdt-owner-fixtures.patch` appends the owner unit module to `composite_value.rs` and adds `crates/sparql-eval/tests/cdt_owned_admission.rs`. Its sources also remain separately available as `cdt-composite-owner-unit-fixtures.rs` and `native-cdt-owner-fixtures.rs`. It reuses existing segmented test support, native reservation factories, and the evaluator's existing `purrdf-alloc-probe` dev dependency. It adds no runtime dependency or semantic feature. The fixture patch was assembled against the shipping owner module; it has not been compiled or run by this helper.

The fixtures cover:

1. Exact surviving native vector/string/box layouts, excluding discarded parser and measuring work.
2. Each real native growth refusal and native box allocator refusal remaining physical, with no later growth after refusal.
3. Resident lexical-error equality, malformed host/direction/escape cases, exact duplicate map-key offset and spelling.
4. Total syntactic ordering, map canonical ordering, nested triples, stable equal keys, and scalar comparator cleanup.
5. Quiet IRI verdict parity and zero allocation, checked destination overflow before admission, and allocation-free deep heterogeneous drop.
6. Real public direct, prepared, governed, and prepared-governed bounded SELECT/VALUES/ORDER entries, malformed composite opaque keys, exact duplicate bag, and tie order.
7. Immutable result and composite payload clones retaining original allocations until the last carrier dies, and malformed parser diagnostics dropping before the original grant.

Qualification still belongs to the single writer/build lane. Neither dry-run application nor this proposal is a compiler/test result, and this unit alone does not establish the complete all-algebra/all-producer #508 contract.
