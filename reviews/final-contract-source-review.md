# Final five-contract source review

Source verdict: PASS for the reviewed settled implementation and repairs. No new
material source defect established. Delivery verdict: FINDINGS-OPEN because the
required qualification and integration remain incomplete. This is not approval
to merge or a claim that any issue is complete.

Reviewed 2026-10-11 in `515-sparql-eval-datalog-per-row-evidence`, HEAD
`ccfac0a5e2387ca9e4fd2a1dd2c4840281734a91`, plus the 20-file uncommitted diff
whose `git diff --binary | sha256sum` is
`642c6d1f9fc83545bf7a5deea2e8161702b87d23b8b54cbe9ce940b2dc782e79`.
The verdict is bound to those inputs; subsequent material changes need an
affected review. No builds, benchmarks, process changes, private inputs, forge
actions or shipping-source edits were performed. Only this report was written.

## Authority and coverage

Read applicable AGENTS, main `.baseline` and `.goals`, all five captured issue
bodies/addenda, `plan.md`, `shape-contract-plan.md` and current `validation.md`.
Applied the Stage completion-adversary distinction between source correctness,
production consumers and complete executed acceptance. Reused the existing
`five-contract-implementation-review.md`, `ordinary-fast-path-review.md` and
`reviews/native-row-control-root-review.md` for unchanged reviewed source and
their attributable focused results; this review does not recreate their cleared
findings or promote predecessor receipts to current whole-contract acceptance.

The unchanged first review covers the core information/frame/dictionary and
acceptance laws, original SPARQL/Datalog propagation, ledger queries, candidate
scope, all three physical shapes, dictionary/span consumers and central planner
wiring. This pass concentrated on their repair dispositions, current native row
representation, ordinary fast paths, native evidence ingress and rowless search,
the actual fused acceptance consumer, and native/WASM test registration.

## Material source conclusions

- The original phrase-bound defect is corrected at its producer home:
  `crates/text/src/dictionary_relation.rs:35` uses
  `TextIndex::phrase_row_bound` (`crates/text/src/index.rs:784`). The prior
  reviewed bound accounts for contiguous logical-position branch products,
  rather than treating physical token count as a bound on hidden-graph alias
  derivations. No new declaration-only workaround was introduced.
- The fused acceptance gap is closed in production source. Public
  `search_scoped` (`crates/retrieval/src/search.rs:548`) enters the original
  execute/adapter/fusion composition. Scoped execution retains complete original
  receipts; `FusionStream::retain_scoped` (`fusion_stream.rs:2679`) imports their
  evidence and sends uncertain observations through the existing contribution
  and frontier machinery. `fuse.rs:365–405` retains both partitions in certified
  order while uncertain rows do not consume established top-k slots.
  `ScopedFusedAnswer::with_accepted_rows` (`scope.rs:303`) borrows those retained
  scored payloads through `QueryEvidence::with_accepted_payloads`
  (`evidence_exec.rs:790`). Acceptance does not invoke a producer or recompute
  scores. Original contributions, records and row identity remain available;
  acceptance annotation/identity is supplied by the shared `AcceptedMerge` law.
- Typed settlement errors and terminal native failure handling remain repaired.
  The additional pull/settle changes in `retrieval/src/execute.rs` preserve
  native search evidence before checking whether a cursor emitted a row.
  Unscoped extraction refuses nonvirtual evidence, including owner-only or
  rank-only default records, instead of discarding it. This includes empty
  producer results and late search evidence observed at settlement.
- `EvidenceInput::bind_native_evidence` (`evidence_exec.rs:332`) adopts the
  original native owner when no dictionary exists and otherwise imports at the
  canonical owner home. Materialized property calls, cursor binding and final
  search reads, and host-shape publication/search composition use that same
  boundary. `QueryEvidence::import`, combination and transformation preserve an
  explicitly retained owner even when the canonical record becomes default.
  They also retain membership, closure and original-rank state. The expanded
  dictionary shortcut remains disabled when external evidence requires the
  ordinary source ingress.
- The native-only shared control is used by both tracked row controls at the
  existing workspace admission home. The public caller-capable wrapper remains
  available. Attachment admits controls before moving resident cells, shared
  clones keep the original grants, and payload destruction precedes grant
  release. The previously reviewed unique-access and last-owner behavior is
  retained. The 40-byte ordinary row and allocation measurements are focused
  evidence, not proof of query throughput.
- `RowBag`'s conservative attachment summary is updated after successful row
  publication/combination, recomputed when mutable resident storage re-enters
  the bag, and preserved by cloning/ordering. Truncation may leave it true,
  which only disables an optimization. Search evidence remains independent.
  Current FILTER, VM, projection, grouping, path and join fast paths use these
  physical absence proofs; semantic default-record equality is not substituted
  for absence of owner/rank/closure attachments. Existing aggregate values and
  original expression/error order remain the computation used by those paths.
- The affected custom-runner targets are explicitly registered with
  `harness = false`: evaluator `evidence_ingress`, text `physical_shapes` and
  `wasm_determinism`, and retrieval `real_producers`/`wasm_determinism`.
  `Makefile` registers the physical shape and actual scoped retrieval/ingress
  consumers for the original WASM runner. No counterpart of the geo zero-test
  registration omission was found. Registration alone proves no runtime result.

## Remaining completion blockers and exact next checks

1. Run the unchanged original 36-lane current/baseline comparison under exclusive
   timing conditions. The predecessor's 20 rejected lanes, including numeric
   controls, remain unresolved until the applicable new comparison establishes
   otherwise. The five-pair approximately -4.22% instruction diagnostic is
   useful diagnosis, not throughput acceptance. Diagnose any rejection at its
   actual computation without changing the benchmark or threshold.
2. Complete the original 78 physical scaling measurements and preserve the 19
   physical correctness controls with source applicability. Source registration
   and correct logical answers do not establish one-pass work or scaling cost.
3. Complete the required 110 scalar plus 15 SIMD portable executions with exact
   terminal counts and current-source applicability. The declared tests and
   earlier native controls do not substitute for actual portable execution.
4. Complete generated/manual claim reconciliation and the unchanged mandatory
   gate, then normal hooked publication, hosted checks, actual integration
   assessment and `ghprsq`. Verify all five closures before counting delivery
   or removing the delivery branch/worktree.

No additional implementation work or new test suite is requested by this source
review. A concrete failure from these remaining checks requires diagnosis and a
bounded production repair before that affected requirement can pass.
