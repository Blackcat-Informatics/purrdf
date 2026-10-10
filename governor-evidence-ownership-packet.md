# Governor evidence ownership packet

Concrete implementation support from the current selected 508 worktree. This is not a validation receipt or an additional review. No source changes, builds, tests or forge actions were performed. Source line numbers may move during implementation.

## Chosen snapshot design

Keep the existing `GovernorState::evidence() -> GovernorEvidence` API and the public borrowed `silenced()` / `expression_errors()` accessors. Change its variable-sized storage so the method is allocation-free, including on a populated sticky operational failure:

```text
GovernorEvidence
  consumed / limits / tripped           stack value fields, as today
  silenced                             immutable SilencedEvidence owner
  expression_errors                    inline SmallVec, capacity ErrorCode::ALL.len()

SilencedEvidence
  Empty                                inline, no allocation
  Shared(Arc<SilencedPayload>)          shallow Clone

SilencedPayload
  records: Vec<SilencedInvocation>      canonical sorted order; duplicates preserved
  lease: opaque admitted owner         payload declared before lease
```

The immutable record payload belongs in the existing core governor/evidence home (`crates/rdf-core/src/governor.rs`), since the core evidence type must not depend on `sparql-eval`. The opaque owner need only keep an already-admitted `Send + Sync` lease alive; it does not expose budget mutation or erase the operational error. The existing evaluator `WorkspaceAllocation` can be passed as that lease. Admission must include the record vector, strings, private payload, Arc control layout and any lease box before constructing them.

Keep the wrapper's storage private. Expose borrowed records and shallow owner cloning; do not expose a raw `Vec`, raw payload Arc, or extraction that leaves its lease behind. **Payload mutation and lease replacement must remain internal to the core owner.** A public `unique_records_mut() -> &mut Vec<_>` permits `mem::take` to extract live records without their lease; a public `replace_admission(...)` permits substituting a no-op owner and dropping the real live charge. Neither is a sound API even when the payload currently has one Arc reference. Only a narrow admitted insertion/replacement operation that cannot transfer raw ownership or release live admission is acceptable. If that operation is not yet available, construct an admitted immutable replacement and atomically swap whole owners; omit the raw mutation fast path. Debug/Eq compare semantic records, not account identity. Empty construction remains allocation-free. Existing ordinary resident query result types remain unchanged; the same snapshot method works with resident/no-op admission.

Numeric error codes have an **exact finite bound of eight**, not a guessed reporting allowance: `crates/xsd/src/value.rs::ErrorCode::ALL` is `[Self; 8]`. Populate the inline small vector in that code order from the fixed atomic counters, excluding zero counts. It never spills because there can be no ninth code. This preserves the current ordered sparse list and avoids `Vec::collect` on every snapshot. No additional resource ceiling is needed.

## Current concrete allocation sites

| Source home | Observed behavior | Required adaptation |
|---|---|---|
| `governor/mod.rs::GovernorState`, around 844 | Fixed arrays/atomics plus `Mutex<Vec<SilencedInvocation>>`; `new` itself starts with empty vectors. Public entries then allocate an Arc for the state. | Keep fixed counters. Admit engine-created state Arc/owner layout before construction through reporting capability. Caller-supplied operation state must use the same admitted record path; do not assume that external Arc ownership makes new record growth free. |
| `GovernorState::record_silenced`, around 906 | Pushes an already-allocated record without admission. | Use admitted ordered record insertion, returning `Result<(), EvalError>`; propagate hard operational refusal rather than pretending SILENT succeeded. |
| `GovernorState::evidence`, around 1141 | Deep-clones SILENT target/message strings, stable-sorts the vector, and collects numeric errors into a new Vec. | Load scalar counters, shallow-clone immutable sorted record owner, build the inline eight-code list. No allocation, sort, resize, formatting or fallible growth. |
| `EvalCtx::record_silenced`, around 1670 | Returns unit and discards no failure because current state method cannot fail. | Propagate record admission failure. Pass the actual context growth capability to the construction/insertion owner. |
| `remote.rs::failed_invocation`, around 953 | Calls `endpoint.to_owned()` and `error.to_string()` **before** the record reaches the state. | Move admission before these actual allocations; post-record insertion admission is too late. Preserve non-SILENT semantics and ensure resource admission errors are never themselves silenced. |
| `remote.rs` SILENT dispatch around 933; `service_endpoints.rs` around 1457 | Records the failure, then produces the specified identity/endpoint block. | Admit/create/record first; only publish the SILENT identity after success. Preserve one record per invocation, endpoint order and frame-role semantics. |
| `update.rs::silence_or_fail`, around 1023 | Copies source IRI and moves an already-created diagnostic message into a LOAD record. | Thread the same admitted record helper/capability through this shared state caller. The moved message may transfer its existing admitted owner; copying the source still needs earlier admission. Do not create an unchecked alternative record path. |
| `rdf-core/governor.rs::GovernorEvidence::retained_bytes` | Measures capacities of Vec/String fields after construction. | Update for immutable admitted records and inline errors. Explicitly distinguish already-leased shared bytes from unleased enclosing publication bytes so final `set_base` does not charge the same records twice. Measurement does not retroactively admit initial allocation. |
| `governor/ledger.rs::ChargeLedger::for_plan`, around 204 | Allocates `nodes`, per-node fuel, rows and cells vectors from `PlanShape::len()`. | Add one admitted construction path for these known-size allocations and snapshot output capacity before evaluation begins. |
| `ChargeLedger::snapshot`, around 262 | Allocates a fresh `Vec<NodeCharges>` with collect. | Fill/move already-admitted output storage at terminal EXPLAIN; do not collect after a failed session. |
| `engine.rs::explain_prepared`, around 2836 | Runs evaluator, then describes registries, calls `ledger.snapshot()`, constructs profile digest and captures governor evidence. | Prepare/admit known immutable EXPLAIN pieces before the run, or grow them on an explicitly ready success path. Failure paths use existing owned records and scalar snapshot only. |
| `engine/graph_build.rs::GraphBuildError::BudgetExhausted` | Several paths use `Box::new(state.evidence())`. | Allocation-free evidence does not make the Box allocation free. Store this fixed/inline evidence directly, or admit/preconstruct its owner before the run and transfer it. |

## Record construction and copy-on-write sequence

Suggested concrete internal interface (names can follow the writer's implementation):

```rust
// Borrowed target; no endpoint/source String is created merely to request admission.
enum SilencedTargetRef<'a> {
    Service { endpoint: &'a str },
    Load { iri: &'a str },
}

fn record_silenced(
    &self,
    target: SilencedTargetRef<'_>,
    kind: SilencedKind,
    message: &impl std::fmt::Display,
    growth: &WorkspaceCapability,
) -> Result<(), EvalError>;

// Existing method name and result type; allocation-free for all states.
fn evidence(&self) -> GovernorEvidence;
```

Actual rendering can use a checked, nonallocating length pass followed by admitted fallible construction, or the existing text-sink law with capacity admission **before each growth**. Preserve the message bytes and full endpoint/source, rather than truncating them to fit an invented allowance. Avoid first building an unpriced `endpoint_text` or diagnostic String merely to pass it to this helper. Already-owned input errors/messages keep their own earlier owner until transferred or released.

Under the record mutex:

1. Check whether the existing immutable record payload is unique. A prior receipt may still share it. An external shared `GovernorState` is an intentional supported operation surface, so `Arc::try_unwrap`/destructive drain cannot be the only snapshot design.
2. Calculate checked allocation layout for the new record and for any needed replacement vector/control owner. If copy-on-write is required, include copies of every existing target/message String, the new vector, and old/new allocation overlap. Earlier snapshots keep their original charges.
3. Acquire real workspace admission **before** cloning, reserving, formatting or constructing the replacement. Do not call `Arc::make_mut` and charge afterward. A refusal leaves the previous admitted record set intact and immediately aborts this operation with the original typed cause.
4. Construct fallibly, insert in `SilencedInvocation::Ord` position without another allocation, and atomically replace the state's record owner. Do not deduplicate identical invocations: the contract is one record per failed invocation.
5. Release old unshared payload/charge only after the old allocation is gone. Shared old snapshots survive unchanged. No snapshot-time sorting is necessary.

Using a unique payload in place is permitted only through a core-internal operation if its current lease covers the intended growth before mutation. Do not export `&mut Vec`, raw record extraction or public lease substitution to implement that optimization. The current `WorkspaceAllocation` is not independently resizable; use a narrow owner operation with genuine growth/transfer support or build an admitted immutable replacement. Never discard the old charge before vector reallocation/copy is complete.

Avoid a strong cycle: record payload -> allocation lease -> account is sound; account must not retain the governor state or record payload back. The state and any result/error/explanation may each own a shallow record handle, all retaining one underlying allocation charge.

## Charge ledger and ordered worker logs

The public EXPLAIN ledger is fixed by `survey.shape().len()`. `LedgerNode` holds a static label, depth and scalar `PlanEstimate`; `NodeCharges` holds static label, ordinal/depth, fixed fuel array, rows/cells and scalar estimate. Cloning those metadata values requires no String allocation.

For `n` plan nodes, admit checked capacities for:

```text
n * size_of::<LedgerNode>()
n * size_of::<[AtomicU64; CHARGE_SCHEDULE.len()]>()
n * size_of::<AtomicU64>()                 // rows
n * size_of::<AtomicU64>()                 // cells
n * size_of::<NodeCharges>()               // preallocated final snapshot
ChargeLedger / Arc / snapshot owner layouts
```

These are allocated-layout expressions, not row heuristics. `record_fuel`, `record_rows`, `record_cells` subsequently update existing atomic slots and need no growth. At terminal measurement, fill the reserved snapshot vector and move it into the explanation while transferring/retaining its charge. A once-only consuming/final snapshot API is appropriate for this private, per-EXPLAIN ledger; retain any resident diagnostic snapshot convenience only through the same construction home. Do not repeatedly materialize identical snapshots for separate receipt branches.

Separate dynamically growing owners exist in `row_checkpoint.rs`: `ChargeLog::charges: Vec<Deferred>` (fuel/scratch/transient/growth pushes), `WorkerLedger::items`, `WorkerLedger::charges`, worker minted values, `ExactDeferral` Arc/Mutex and committed chunk vectors. Each real reachable push/clone/collect must have before-growth admission, or a certified fixed capacity supplied before the loop. `engine.rs::eval_ctx` currently forces bounded operational evaluation sequential, and `RowCheckpoint::for_rows` selects its sequential mode when not forked. That avoids some worker allocations; **it does not prove every `ItemLedger` construction/deferral path is unreachable** (`ItemLedger::for_items` keys off governor presence). Wire or prove the actual remaining routes; do not waive their ownership because there is one local build lane.

Keep deterministic ordered commit, fuel/scratch semantics and normal resident fork behavior unchanged. Saturating report counters are existing governor semantics; workspace capacity calculations must still be checked. This packet does not require replacing those counters with a new scheduling law.

## Publication and sticky-failure call-site map

The current `finish_governed_fallible_query` around `engine.rs:4026` reads `state.evidence()` before the status match, again on complete publication, and again in multiple retention/final-read error mappings. Its first workspace-failure arm also calls `state.evidence()` after admission has failed. Prepared ingress and `preflight_governed_fallible_view` do the same. An admission attempted here to cover a just-created receipt would necessarily be too late.

Use the new allocation-free snapshot once for each actual terminal observation, then move or shallow-clone that value into the selected branch:

```text
finish/join all participating execution workers
read current fixed counters + shallow record owner
check workspace's first typed failure and backend operation status
select the already-defined root-cause precedence
compose GovernedEvidence { view, governors } on the stack
move into success/Query/Operational/BudgetExhausted outcome
```

Where result retention/drain can still fail or charge before final publication, take its **final** scalar snapshot after that phase (the SILENT owner remains shared); do not derive a verdict from one snapshot and report different concurrent values from another. `resolve_governed` already documents the one-reading requirement. Failure handling itself performs no reserve, grow, collect, deep clone, stable sort, String rendering, Box creation or Arc creation.

Specific call families to migrate/use consistently:

- `engine.rs`: new governed state entries around 1534/1578/1984; EXPLAIN state around 2856; `resolve_governed`/`materialize_governed`; `finish_governed_fallible_query`; `reserve_governed_reporting`.
- `engine/prepared_fallible.rs`: reporting/initial state, shared-operation entry, workspace-admission mappings and `preflight_governed_fallible_view`.
- `engine/graph_build.rs`: budget snapshots at admission, stage finish and publish; boxed evidence owners described above.
- `governed.rs`: `GovernedEvidence<Evidence>` and resident complete/budget outcomes already carry `GovernorEvidence` by value; its new shallow record storage makes these clones safe without introducing a second result type.
- `interned.rs`: resident/in-operation governor evidence carriers use the same snapshot law, including a receipt returned after a scoped visitor.

`RetainedEvidence::backend` deliberately keeps provider-owned evidence inline to report an initial refusal without allocating. Engine-added governor records cannot borrow that guarantee: they must carry their own already-admitted record owner. Conversely wrapping a populated failure receipt in `RetainedEvidence::new` creates an Arc and lease Box; do not do that at a sticky failure boundary. The proposed inline scalar + immutable admitted record design composes safely with the existing generic error evidence field without creating a new outer heap owner.

An initial reporting refusal can still return `GovernorEvidence::new(limits)` with empty records and inline empty error list. `reserve_governed_reporting` currently builds a stack-only fresh state for that zero snapshot. Preserve this behavior. A supplied state that already has SILENT/F&O history must return its populated allocation-free snapshot instead of replacing it with zero to avoid allocation.

Final retained result/partial/evidence/explanation lifetimes must keep the shared record lease after contexts and read sessions die. `CompleteSparqlResult::into_parts`, receipt clone, `FallibleSparqlError::clone` and borrowed/extracted evidence may all outlive the originating session. The immutable record wrapper must retain the lease on its own, not rely on a temporary `_reporting` variable or only on the result sibling.

## Concrete consumer adaptation

The borrowed slice APIs already serve CLI (`crates/cli/src/governors.rs`), C governor encoding (`crates/rdf-capi/src/governor.rs`), WASM query wrappers (`crates/rdf-wasm/src/query.rs`), Python query wrappers, numeric conformance and service resolver tests. Their serialized bytes/error ordering should remain identical.

Direct Vec moves that need adapting include `sparql-eval/src/remote.rs` test helpers around 1547 and 2456/2479, and `rdf-capi/src/query/division_tests.rs:194` returning `evidence.expression_errors`. Prefer borrowed slices or retain the new owner in test receipts; an explicitly requested caller/test copy can use `.to_vec()` outside engine publication. `sparql-eval/tests/support/governor_counts.rs` accesses fields directly for diagnostics; maintain slice-like `.is_empty()`/Debug or use accessors. Check equality assertions on wrapper vs Vec by comparing borrowed slices, rather than adding a hidden deep-copy conversion inside engine paths.

## Focused executable acceptance

These cases belong to the coherent final focused check, not repeated full gate invocations:

1. Produce several real `SERVICE SILENT` failures with long endpoint/message text and all available numeric F&O codes; force a subsequent lazy storage/workspace refusal. The Operational error contains exactly the pre-refusal sorted SILENT records and sparse code-order counts, preserves the exact original typed root, and allocates nothing while capturing/mapping/cloning the snapshot.
2. Capture receipt A from a shared operation state, continue the state to receipt B with more SILENT records, then force refusal while admitting another record. A remains byte-identical and alive; B contains every admitted invocation once; refused growth neither allocates the refused record nor returns a successful SILENT identity.
3. Hold only governor evidence, an Operational error clone, or an extracted complete receipt after result/context/session destruction. Allocated record bytes remain charged once until the last actual record owner dies; dropping one clone does not release live capacity and does not create a strong account cycle.
4. Exact inclusive record/vector capacity succeeds; one-byte-less capacity fails before the vector, target or message allocation. A snapshot of the populated state still works after that failure. Include failure at COW replacement while an earlier receipt remains retained.
5. EXPLAIN over a many-node plan retains its exact NodeCharges and full registry/profile metadata. Fail storage during measurement: no new ledger snapshot/descriptor/profile/error-format allocation occurs on the failure path; the typed Operational outcome outranks evaluator/governor fallout.
6. Resident API/serialization/goldens remain identical, including duplicate SILENT invocations, LOAD's shared state path, F&O zero filtering, plan pre-order and full profile identity. No silent record loss or shortened messages.

Existing allocation probe and actual segmented/storage evidence can prove these boundaries. Source inspection here does not establish they pass. Any remaining post-failure Vec/String/Arc/Box creation or unpriced pre-record creation is a concrete unfinished owner, not a permitted exception.
