<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Preliminary independent native structural-cost review

This is an EARLY implementation advisory, not Task 1 completion review, PASS,
or a claim that primary Rust performance has been qualified. No builds, tests,
timing measurements, environment changes, forge actions, or shipping edits were
performed. Only this review artifact was written. The source writer was active;
the observations below cover the files read, not an immutable final candidate.

## Evidence identity and limits

- Baseline and candidate HEAD: `18489a91cb452b1773c60b469d4c30dc33a9a4a4`;
  task-owned clean baseline tree supplied by root:
  `5d150fa4076cc75e293b517f79dd080b6ea807ed`.
- Plan SHA-256 observed: `8627bc25be581a26269cf8ad5cbe9f826d2917f6168d8cb5e35c71e132ca01f6`.
- Tracked binary-diff SHA-256 sampled after reading:
  `bbf7706cc49ccfcbe5bc7888038e8020870e076dcd2a74cf84dfa9ef283932f8`.
  This digest excludes untracked files and is a read-time binding, not an atomic
  source snapshot. The sampled untracked `rdflib.rs` SHA-256 was
  `590cf997f6cdb9cd00d818aaef4d7a749ab094b7a0a860aab6fb4c46144671ee`.
- Read governing AGENTS, `.baseline`, `.goals`, current plan and validation
  index, relevant tracked algebra/evaluator diff and probe source/receipts.
  No more-specific governing files were found under affected crates.
- Existing `t1-native-cost-{base,candidate}.log` measurement records visibly
  agree for all seven workloads. Complete log files differ in build output.
  Reported layout: GraphPattern 144, Expression 64, Query 264,
  PreparedQuery 368 and NodeRef 16 bytes on this probe target.
  All recorded parse/clone/drop/prepare/reuse allocation measurements and
  retained query/plan bytes agree. These are implementer-run receipts,
  independently read, not runs performed by this reviewer. Their final source
  attribution is still required. The probe has no timing instrumentation.

## Source-supported findings

No concrete extra ordinary allocation, recursive traversal, per-row collection,
or whole-tree pass was found in the inspected native delta. The principal
structural repairs are appropriate; do not manufacture a rejection merely
because a new enum arm exists.

1. Ordinary parser entry points instantiate `Parser<false>`; only the named
   contextual entry instantiates `<true>`. The parser gained no runtime mode
   field. Constant-false `group_join` selects the original `join`; strict BIND,
   projection/GROUP alias checks, scope checks and ordinary star census retain
   their old operations. Compatibility SAMPLE lifting and syntactic MINUS
   census are const-gated. `sampled_projections = Vec::new()` does not allocate;
   its ordinary iteration is over a provably empty collection. This introduces
   no source-level native collection growth, but removal of residual loop/drop
   scaffolding belongs to compiled-code proof if claiming instruction equality.
2. Apply policy is boxed. Its own allocations and policy-variable/condition
   visits occur on Apply nodes; ordinary parsing does not construct Apply.
   Added clone/drop/walk/retained-size/property-plan/governor/service/BGP arms
   preserve the operations of pre-existing variants. Ordinary hash scripts
   still feed named variants and the same fields, not shifted enum ordinal
   bytes. No metadata storage was added to PreparedQuery or EvalCtx here.
3. Ordinary clone uses `clone_tree::<false>`; the EXISTS mapper callback is a
   function item and is not invoked by the false branch. Substitution's
   `<false>` specialization retains the original Project narrowing and
   recursive operation sequence. The new map/retention sets, row clones,
   optional condition and policy allocations in its Apply arm are contextual
   costs, not extra work on an ordinary Project or Join.
4. Native LATERAL and correlated evaluation select `<false>` application and
   substitution. The property-function interception, endpoint admission,
   deferred-site logic, driver loop, compatibility scan, merge, charging and
   row allocation remain the original native algorithm. Compatibility retry
   work is reached only through `eval_apply`/`<true>`. `engine.rs` adds typed
   contextual prepare/execute entries; ordinary prepare/prepared execution did
   not acquire a contextual compiler call or plan-wide mode check.
5. Mutable expression traversal moved from eval's substitute module into
   algebra's walk module with the same Vec initialization, reversal, pop and
   operand pushes. This is first-party relocation of an existing helper; its
   delta is not copied upstream implementation. Cross-crate generic
   monomorphization makes equivalent work plausible, but is not itself an
   assembly comparison. This review does not certify provenance of all new
   compiler bodies.

## Concrete unresolved cost-proof points

These are evidence gaps, not observed regressions or speculative defect claims.
Each identifies an actual shared caller and the potential extra operation to
exclude in final qualification.

- **Shared enum dispatch:** `eval_node` evaluates every ordinary algebra node;
  NodeRef's child/variable visitors, `take_pattern`, `assemble_pattern`,
  `Script::pattern`, governor analysis and property planning also dispatch over
  GraphPattern. A new arm can change branch chains/jump-table bounds even with
  identical old arm bodies. Boxed policy and equal allocation counts do not
  prove unchanged instruction paths. Compare old-variant reachable dispatch
  paths, including common BGP/Join/Filter/Project and correlation, and establish
  unchanged payload offsets/alignment. A larger unreachable table alone is not
  an extra executed operation; an added executed compare/load/call would be.
- **LATERAL call boundary:** `eval_lateral` changed from inline-never body to
  inline-always wrapper invoking inline-never `eval_application::<D,false>`
  with an extra `None` argument. Exclude a surviving wrapper call, policy
  argument setup, policy test or larger ordinary stack frame in the relevant
  monomorphized caller/callee. The source constant resolves the algorithm but
  not ABI/codegen overhead.
- **Correlated site construction:** `eval_substituted_with::<D,false>` now
  wraps the old sites value in `Some` through `.then`, then borrows it through
  `as_ref().expect`. Exclude a surviving tag store/test or extra move/frame
  growth. Ordinary EXISTS/LATERAL calls still perform exactly one
  `nested_sites` operation at source level.
- **Clone and relocated walker:** demonstrate that false clone specialization
  introduces no mapper dispatch and native expression mutation retains its
  prior call boundary/work. Shared visitor dispatch should be covered here,
  not inferred from equality of seven allocator windows.

The available `native-cost-base.asm` begins with a single `.text` symbol and
no named relevant function sections were found by symbol-header search; no
candidate assembly or paired control-flow report was present in the inspected
raw inventory. It therefore did not establish any of the paired compiled-code
claims in this review. A huge unsymbolized dump is not a reviewed mapping from
production caller to native specialization.

## Necessary final proof, proportional to this delta

1. Freeze the final source identity, probe source, compiler/target/profile and
   effective flags for both base and candidate. Preserve all seven untimed
   structural receipts against that identity. Include offset/alignment or
   compiler-layout evidence for changed shared enum/work-list types if layout
   claims exceed their already measured sizes.
2. Write a caller-to-specialization matrix covering ordinary parse/update,
   prepare/admission, clone/drop/hash/walk, prepared evaluation, EXISTS/LATERAL,
   native prebinding/SHACL and property-function interception. For unchanged
   branch bodies, a precise source operation argument suffices. For the four
   codegen-sensitive points above, provide attributable paired IR/assembly and
   an explicit reachable-path argument. Exact whole-binary equality is neither
   needed nor expected; normalize addresses/symbol identity without erasing
   loads, tests, calls, spills or allocation sites.
3. Verify strict ordinary entry points cannot newly reach contextual admission
   or compiler work; keep typed contextual algebra execution separately
   attributed. Extend untimed probes only where a touched path is uncovered
   (not arbitrary whole-workspace repetition). Current cases cover native BGP,
   join/filter, OPTIONAL, UNION/DISTINCT, grouping, MINUS/EXISTS and LATERAL;
   they do not themselves cover update, SHACL substitution, parameterized
   preparation, property-function interception, hash token work or all
   governor/configuration paths.
4. Explain any surviving extra ordinary executed operation and repair it before
   claiming the user-required no-degradation proof. Matching allocations and
   asymptotic complexity alone cannot discharge added CPU work. No timings on
   this host. Re-review only final relevant changes and unresolved proof gaps.

Conclusion: the inspected design isolates compatibility costs coherently at
source level. There is no demonstrated native regression in this preliminary
read; the compiled/native-path proof remains NOT MET, as the validation index
already states. Task 1 remains under development and requires its requested
final independent review after implementation and proof are complete.
