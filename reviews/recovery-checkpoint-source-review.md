<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Recovery checkpoint source review

VERDICT: PASS for the bounded source corrections below; BLOCKED for complete
capacity/sparse-storage delivery. This consolidates the root's actual source
reviews, not an independent fresh review of every file in the checkpoint.

Reviewed source and dispositions:

- `crates/lex/src/allocation.rs`: the private inline byte arrays preserve full
  u128 attempted/limit values through const accessors. No allocation, unsafe code,
  packed references or narrowed counts are introduced. Debug and Hash retain the
  original logical field order and values. Constructor callers must propagate the
  actual producer error; this review does not replace the complete ingress audit.
- `crates/rdf-core/src/diagnostic.rs`: a single optional attachment owner preserves
  presentation and original StorageError together. Debug/Hash retain logical
  semantics, Error::source borrows the retained original cause, and memory cloning
  and retained-byte accounting include the actual attachment and owned payloads.
  Ordinary diagnostics regain one optional pointer. Cold storage-only attachment
  payload grows from33 to120 bytes; do not claim universal allocation-byte savings.
- `crates/rdf-core/src/ir/paged/graph_index.rs`: CSR offsets describe the combined
  resident postings buffer, so usize removes an unjustified global u32 narrowing.
  All four derivation callers retain the same page ordering and persisted format.
  Native64 offsets grow4 to8 bytes; wasm32 offsets remain4. The new actual-producer
  test checks usize offsets and wide PageIds, not an actual four-billion-entry run.
  This correction does not establish fallible physical allocation for every CSR
  allocation or complete capacity ingress coverage.
- Slice error helper and six RDF callers borrow the diagnostic while recovering
  its original storage cause. Fallible dataset union/report generation propagates
  through CLI ingestion, rule output, validation and emission. ShapesError uses
  the existing CLI runtime-message boundary; contextual imports retain context.
- Validate rule adapters remove identity conversions. Certificate rendering/parsing
  preserves u128 capacity counts; new tests cover beyond-u64 and u128::MAX values.
  Parsing rejects invalid attempted<=limit records and extra arguments.
- The existing Datalog rules_runtime harness receives the original host-dictionary
  tests through one crate-local shared fixture, preserving high IDs, ownership
  pins, Supported/Opposed evidence and unchanged source identity assertions.
  Registration is not a claim of runtime execution.

Evidence read: diagnostic5, CSR4 and component-layout probe receipt
`recovery-diagnostic-owner-csr-runtime.log` terminal0; final Debug/Hash/cause
control in `recovery-workspace-shapes-callers-strict.log`; latest workspace
all-target strict `recovery-workspace-cli-callers-strict.log` terminal0. Measured
ordinary layouts are RdfDiagnostic96, evaluator/execution/fusion errors96 and
SearchError112 bytes on the measured native target. No timing win is claimed.

Current CLI/certificate/native-and-WASM Datalog batch29668 is still running when
this report is written. Its terminal results, normal hooks and publication must
be recorded separately. Broad fixture adaptations are covered by compilation,
not a blanket source approval from this bounded review. The mandatory full gate,
full host acceptance and enforced capacity-panic census remain incomplete. Manual
source inspection is not an enforced census. Tooling remains frozen. No issue
closure, PR readiness or full-contract completion is established by this report.

Subsequent runtime readback: root read
`recovery-cli-certificate-portable-datalog-runtime.log` terminal_status=0.
The CLI exact-message/runtime-exit refusal control passes1/1; the full-width
capacity certificate round-trip control passes1/1. All six existing
rules_runtime cases pass natively and through the unchanged WASM runner
(six executed, zero filtered, on each target), including the shared host-ID and
evidence case. This clears those pending focused executions only. Full gates,
host acceptance and the enforced census remain incomplete.

Subsequent C source review: `crates/rdf-capi/src/version.rs` correctly describes
the additive StorageError15 within the existing unshipped ABI0.10 bump, limited
to Shapes entry points. The actual `PurrdfError::shapes` storage arm routes the
original cause through `from_diagnostic`, and real SHACL callers use that seam.
Other diagnostic callers retain their existing operation status and typed JSON.
The wildcard-free status control pins prior0..14, appended15 and Panic100.
No material source finding in this scope; generated header and C package batch
31032 are pending at this review point, and are not approved by source inspection.

Generated projection readback: root read the actual header diff, which contains
only the matching ABI prose and appended PURRDF_STATUS_STORAGE_ERROR=15.
Existing prototypes and prior enum values are unchanged. The original generator
and exact-artifact header check pass; four generator self-tests pass (owner
reported, retained batch log). C smoke remains live, with five harness controls
already passed and its actual C ABI workload still running. No complete C package
pass is inferred until the batch's required suffixes finish.

Final C runtime readback: root read batch31032's raw terminal_status=0 suffix.
Unchanged C smoke6/6 passes, original Shapes capacity diagnostic1/1 passes and
append-only ABI status1/1 passes. The two header/prose files now have scoped C
acceptance; their normal hooked commit remains pending memory admission. This
does not clear the complete issue's full gate, other host acceptance or census.

## Subsequent runtime receipt

The original batch 29668 subsequently completed with terminal 0 in
`recovery-cli-certificate-portable-datalog-runtime.log`: CLI original-storage
message/runtime-exit control 1/1, full-width DL certificate roundtrip 1/1, and
existing Datalog `rules_runtime` 6/6 native plus 6/6 actual Node/wasm32 PASS.
The shared host control retains its three original guarded/evidenced scenarios.
This updates the pending execution state above, without promoting the bounded
review to full delivery or replacing required full gates and host acceptance.
