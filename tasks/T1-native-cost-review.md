<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 1 independent native-cost review

Final disposition: **Native structural-cost requirement MET for the frozen
37-file manifest and paired extended-base/thin-Apply artifacts.** Demonstrated
dispatcher/caller regressions were repaired and verified below. Earlier NOT MET
sections are preserved attempt history, superseded by the final native argument
and disposition. This is a native cost review, not an all-task completion audit.
No timing allowance or threshold is used.

## Source and artifact identity

Synchronized candidate HEAD and independently verified clean detached baseline:
`863c75e2c1e13001bcbe025820d257bd0ee49c4f`, tree
`42cce1fededb04e9a7283bfa589b61e1ca34203c`. The sampled tracked candidate diff
SHA-256 is `6ed6e019d4e602ecb614f50fc6c8a2ef8b7249ce5de8e67e87e1f435b633c298`.
Native files inspected at this sampling:

| File under crates/sparql-eval/src | SHA-256 |
|---|---|
| binop.rs | 418ea9cab54b0b22928c53de0414c046d2c47ae9975865abceaf4c9f8b73f167 |
| eval.rs | f964ecb1bc46414f4238b17b35bb0fc0bca379bdda840fb3f2f92b447bf046cb |
| modifier.rs | 75e6e0a82eecc0bb42356927e1a019c175abd575824b1cecf2f2ffdd2d115ad9 |
| engine.rs | 83aff139dc7eca18a6dbed1e8b7ae26ae4b992d7a491b9c59437468de840f9e9 |
| expr.rs | 580844d88b58cfaea9ead63b8ac70805d2530da6eb58ee3729e1c58860c1fb85 |

The writer subsequently reported a contextual-only pointer-key helper cleanup;
final artifacts/source binding must be refreshed. Historical 18489 artifacts
were preserved and are not used as current-base proof.

Current inspected pair: `raw/native-cost-{base,candidate}-863c75-ir`, the
corresponding `-sections` directories and inventory logs, and
`raw/t1-native-cost-{base,candidate}-863c75.log`. The build logs identify the
Stage nightly-2026-09-14 x86_64 toolchain executable. LLVM triple is
`x86_64-unknown-linux-gnu`; effective rustc invocations use opt-level 3, thin LTO,
codegen-units 1, strip=symbols, `-D warnings`, target-cpu=native and
force-warn=unused_crate_dependencies. Those effective invocations, not the
standalone probe manifest's fat/none profile spelling, govern these artifacts.
Exact rustc/LLVM version and resolved native target-feature identity remain to
be recorded by the writer in the final attribution. No unsupported compiler
identity is inferred merely from a toolchain directory name.

Read-only reviewer work: source/base comparisons, existing emitted IR/assembly
and receipt inspection. No builds, tests, timings, environment mutations,
shipping edits, commits, forge actions or delegation. Only this required report
was written; no temporary probe/debris was created.

## Verified source argument and structural receipts

Unit RowDelivery and false constants exclude contextual consumer callbacks,
row clone/one-row Vec delivery, yielded-block collections, witness Slice/cap
planning, current-row reset, contextual aggregate scan and retry policy work.
Native Join/Minus wrappers call the original kernels; native MINUS's extracted
inline helper retains the same shared-column/domain loop. Filter/Extend/Project
sequence helpers retain their row work; Project now borrows its sequence and
retains the original lifetime/drop scope. Variable GRAPH's extracted helper
uses the same native output Vec across all graphs and the same schema clone,
compatibility and append/resize logic. GroupDomain unit calls the original
shared-blank visibility home. Ordinary algebra admission specializes the
existing height/structural traversal rather than adding another tree scan.
Apply/VarPairs were appended, keeping old variant source order. Boxed policy
does not enlarge the measured GraphPattern. None of these source observations
alone proves unchanged machine-code calls, spills, alias optimization or frames.

The earlier LATERAL width-read drift is resolved in source: the shared helper
accepts a supplied prefix width and native application reads its schema width
once before the output loop. Final compiled proof still needs to bind that fix.

Independently compared current-base ten-workload receipt files: exact equality
with diff exit 0. They include parse/clone/drop/prepare/ordinary-algebra-admit/
prepared reuse allocations, retained bytes, layout, output row counts and
metered resource vectors. Layout remains GraphPattern144/Expression64/
Query264/PreparedQuery368/NodeRef16. The two added GRAPH workloads exercise
fixed and variable named graphs; the tenth exercises BIND/order/slice. Probe
source was read; no clocks/timing instrumentation is present. This is bounded
structural evidence, not CPU instruction or all-input equivalence.

## Blocking compiled finding: native tail dispatch is lost

The production RdfDataset base dispatcher is
`raw/native-cost-base-863c75-sections/section-0054.txt`; candidate unit dispatcher
is `raw/native-cost-candidate-863c75-sections/section-0060.txt`.

Both have three 96-byte LLVM allocas, two saved-register pushes, `subq $312,
%rsp` and CFA336. However their **executed call boundaries differ**:

- Base ordinary Graph/Join/Filter/Project/Union/Minus arms restore the 312-byte
  stack reservation, pop saved registers, then `jmpq *callee@GOTPCREL`.
  Examples: base section lines 574 (Join), 608 (Graph), 696 (Filter), 711
  (Project), 723 (Union), 558 (Minus). The dispatcher frame is gone before
  those recursive operators run.
- Candidate Graph loads the callee address at line 602; Join similarly loads
  it earlier. Filter/Project/Union load it at 665/674/679 and branch to common
  call blocks. Candidate lines 682-688 (`.LBB182_36`) move arguments, execute
  `callq *%rax`, then jump to the return block. The analogous Project block
  has the same live-frame behavior. Stack restoration occurs after the callee
  returns. Candidate IR is an exported sret/personality definition, while the
  base is internal fastcc; these ABI facts must not be normalized away.

This is a concrete ordinary native cost/call-depth regression, not an invented
risk: it adds call/return work and keeps the dispatcher frame live across the
operator's child recursion. Equal prologues and allocator counts do not prove
equal recursive stack use. Old-tag dispatch arithmetic and payload offsets
were preserved in the inspected opening IR, but do not resolve this finding.
The implementer and parent were notified with exact sections/lines. Repair the
native unit boundary and produce final attributable emitted proof, or provide
actual final linked evidence of the transformation; do not assert a linker
will remove a call/return without evidence.

Other inspected frame observations, which are not independent cost PASSes:
Graph base65/candidate90 preserves six pushes/sub1224/CFA1280; Project
base68/candidate82 has stack reservation728 versus696; Group base66/candidate92
has2520 versus2104. No standalone append_graph_rows/merge_application_row/
eval_project_sequence helper call was found in the inspected native bodies;
expected iterator-closure symbols remain. Smaller frames alone do not establish
the entire operator's instruction/branch equivalence.

Additional bounded compiled checks: ordinary eval_evaluated_with base52 versus
candidate66 retains six pushes/sub504/CFA560. Ordinary correlated substitution
base62 versus candidate79 retains six pushes/sub264/CFA320; candidate79 is
internal fastcc with no consumer ABI parameter and no RowConsumer/yield/witness
body found. Native LATERAL base59 versus unit application candidate76 has
reservation1560 versus1432. These observations narrow the unresolved sites;
none retracts the dispatcher's demonstrated loss of tail calls.

## Remaining final qualification obligations

1. Repair and re-review the native recursive dispatcher/caller ABI, tail calls,
   frame lifetime, spills and call depth at final source identity. Inspect
   actual machine paths, not just prologue size or summed LLVM allocas.
2. Complete the paired native operator argument for Filter/Extend/Project/
   Slice/Union/Dedup/Join/Minus/Graph/Group/LATERAL and native correlation. Keep
   extracted-helper return/copy, cleanup, loop width and alias consequences in
   scope. Unit ABI erasure/branch absence must be attributable to the final
   monomorphizations; contextual active symbols are not native evidence.
3. Cover old enum visitors/traits/hash/drop, planning/governor/service walks and
   ordinary admission/prebinding/SHACL paths. Unchanged arm bodies and operation
   sequences can supply source proof where sufficient; altered dispatch/layout/
   ABI sites need compiled proof. Do not invent another whole-workspace ceremony.
4. Discharge COUNT(DISTINCT *)/shared-blank visibility, SHACL substitution and
   property-function interception with bounded untimed witnesses or precise
   source/compiled arguments. The ten probe inputs do not cover those paths.
   Verify required alignment/payload offsets/work-list layout if relying on
   layout claims beyond measured sizes.
5. Supply wasm thin-dispatch frame/call-depth evidence for changed shared
   boundaries. X86-64 equality is not wasm shadow-stack proof. Native source
   invariants extend algorithm reasoning, not target-specific spill/frame facts.
6. Pin final full source/probe/compiler/effective flags/target identities and
   refresh affected receipts after source fixes. No zero-case inventory result
   is a pass; use preserved full sections rather than lossy opcode histograms.
   Retain calls, loads, branches, offsets, spills and exceptional cleanup when
   normalizing symbol names/addresses. No timings or numeric cost allowance.

Review remains active. The native-cost requirement is currently NOT MET by the
inspected candidate, irrespective of semantic/clippy/test successes.

## First native-boundary repair recheck

Distinct attempted-repair evidence was read from
`raw/native-cost-candidate-863c75-native-boundary-{ir,sections}` and its inventory
and untimed receipt logs. Receipt diff against current base again exits 0.
Tracked source diff sampling: SHA-256
`10d3b888fd42ca938d5acdd42693b7257978e234e8680192e74794697121de20`.

Attempted-repair section0067 now names `eval_node`, but remains an exported sret
definition with personality. The same Graph/Join/Filter/Project/Minus/Union
callee address loads and shared `callq *%rax` blocks remain (lines683/706 and
other blocks); no callee tail `jmpq ...@GOTPCREL` restoration was found. Its
312-byte dispatcher reservation remains live during recursive operator calls.
The first attribute-boundary change therefore **does not resolve the finding**.
Both writer and parent were notified. An IR instruction marked `tail call` is
not machine-code evidence that the backend emitted an actual tail jump.

## Independent root-cause analysis after two failed boundary placements

The paired modules are comparable emitted backend stages: their eval LLVM
ModuleIDs name the linked dependency object, each has the corresponding emitted
machine `.s`, and actual compile logs use opt3/ThinLTO/codegen-units1. The source
and compiler metadata match: both eval IR files identify
`rustc version 1.100.0-nightly (4b6d04e70 2026-09-13)`, target triple x86_64,
CPU znver5 and the same expanded target-feature list. PIC2/PIE2/GOT, uwtable2,
frame-pointer1 and debug-version flags also agree. This is not a comparison of
pre-LTO base IR with optimized candidate assembly.

An important preservation gap was found: the staged base file
`qualification_454_native_cost.0heaqv7sw9deez0njj2jbrok3.rcgu.ll` is a 23-line
allocator shim, **not the main caller module**. The actual main is still
readable at the base build's recorded output directory:

`/opt/.cargo/slots/b11c4eccb0eb98a3/0/build/release/build/qualification-454-native-cost/90a95a377902a98a/out/qualification_454_native_cost.qualification_454_native_cost.cc427bdd6978a017-cgu.0.rcgu.ll`

Writer was asked to preserve its `.ll` and `.s`; this reviewer did not copy or
mutate build outputs. The retained base eval LLVM SHA-256
`ac3de5802a2cec7b21e8b03ad524aa9252f2b626199658b8e30c752fadf3cac2`
matches that exact current build-output file, excluding an accidental older
library-artifact comparison.

Caller/retention evidence:

- Actual base main has its OWN internal fastcc `eval_node` definition at line
  76506, with the qualification crate suffix in its symbol. Its local
  eval_evaluated closure calls it at 85703/85819 using fastcc. Base library
  likewise has an internal fastcc definition and local caller, not an exported
  symbol needed by main. Neither is noinline. LLVM may promote the private
  result-pointer ABI because these definitions have only local callers.
- Attempted-repair candidate main instead declares the upstream eval_node at
  122247 and its local closure calls it at 86008/86124 using the external sret
  ABI. Candidate library defines that symbol externally at 153980 with
  attribute #7 containing **noinline**, plus sret/personality; its local
  closure also calls that same external symbol at 489853/489962. Therefore
  ThinLTO cannot make this definition internal while main still refers to it.
- Both failed attempts placed noinline on the static ordinary dispatcher home
  (first generic unit specialization, then native wrapper). Moving that
  attribute merely moved the externally retained home; the first repair did
  not restore the base's consumer-local monomorph/caller arrangement.

The **observed cause** is cross-module symbol retention and the resulting
external result-pointer ABI, which prevents the candidate's actual tail
dispatch. The inference explaining its origin is that explicit noinline
changed monomorphization/import eligibility/ownership compared with the base
default attribute. These final artifacts establish the resulting caller graph;
they do not expose every intermediate compiler pass, so no particular
unrecorded pass order is claimed. Source `fn` privacy alone never establishes
LLVM internal linkage for a generic function used by another crate's
monomorphization.

Evidence-based repair recommendation sent to writer: restore the BASE default
attribute on native eval_node (remove its explicit noinline), retain inlining
of the shared static body into that home, and keep the active contextual
boundary separately out-of-line. This changes the property directly implicated
by both failed retention graphs without copying the evaluator's match into an
alternative implementation. It is a recommendation, not a verified fix. Check
that final main/library graphs again have appropriate local/imported
internalized ordinary definitions and that their actual assembly restores the
base tail jumps; if the compiler still retains an external definition, inspect
its remaining caller/symbol reason before another source change. No linker
repair is inferred and no ABI distinction was normalized away.

## Restored-default boundary: internalization repaired, closure cost remains

Read `raw/native-cost-candidate-863c75-default-boundary-{ir,sections}` and
inventory. Default native wrapper disappears through inlining; its shared
match is outlined as `eval_node_with::<D, ()>::{closure#0}`. Library section0130
is internal fastcc and actual Graph/Join/Filter/Project/Minus/Union tail jumps
return. This confirms that the attribute/retention repair addresses the
previous exported-sret tail-dispatch cause. Do not treat disappearing function
names as proof: the native caller and capture body still change.

Actual main caller comparison against preserved complete base:

- Base `-complete-sections/section-0122.txt` eval_evaluated closure uses four
  pushes/sub56/CFA96. Its ordinary path restores its frame and tailjumps
  eval_node at line235; the scope-guard path calls it at255, as expected.
- Candidate `-default-boundary-sections/section-0193.txt` caller uses four
  pushes/sub72/CFA112. Its IR materializes a 16-byte evaluate closure, stores
  pattern/delivery captures, and calls the outlined match closure on the
  ordinary path at241 (also the guarded path at276). It keeps the added capture
  storage/frame live instead of making the base ordinary tail transfer.
- Candidate library outlined match0130 (main counterpart0194) receives a
  capture pointer rather than pattern directly and begins by loading pattern
  through it before reading its tag. That is a concrete extra native load.

Thus the restored-default attempt is a meaningful partial repair, but **native
cost remains NOT MET** due to closure scaffolding/caller-frame lifetime.
Writer and parent received the evidence. Recommendation grounded in that exact
capture/caller graph: remove the `let evaluate = |ctx| match` capture home and
place the one shared bare match in a named inline-always function receiving
pattern and static delivery directly; retain postdelivery outside the match.
Ordinary default wrapper can then receive the same direct match after static
inlining, while contextual active execution keeps its separate out-of-line
boundary. This shares one evaluator implementation; it does not justify
duplicating its match. Verify the resulting actual main and library caller
graphs/frames/tails before claiming the repair.

## Direct-argument repair: caller restored, Apply body leaks into dispatcher

Read the distinct direct-dispatch IR, full sections and inventory. Actual main
section0194 restores the base caller's four pushes/sub56/CFA96; ordinary transfer
restores the frame then jumps to direct-pattern native eval_node. No evaluate
capture storage or capture-pointer pattern load remains. This discharges the
specific former caller/capture symptom only.

Actual main native dispatcher section0187 is internal fastcc but now has five
pushes/sub928/CFA976. The correct comparison is complete BASE MAIN section0119:
one push/sub144/CFA160. Base library section0054's two pushes/sub312/CFA336 is a
different home and cannot stand in for this consumer-local comparison.
Candidate section0187 IR opens with Lift104/result allocas and contains
eval_apply.exit blocks, reduced/group-domain evaluation and its unwind paths.
Those are directly attributed inlining artifacts of binop::eval_apply, rather
than speculative costs inferred from allocation counts. Source binop.rs:623
declares eval_apply with the default attribute; inline-always eval_apply_with
calls it on the unit branch, allowing its body into this native dispatcher.
The entry reserves the enlarged frame before reading any variant tag, so
ordinary native nodes incur its stack reservation and extra saved registers
even though ordinary parsing does not produce Apply.

Native cost remains **NOT MET**. Evidence-based recommendation: retain a real
out-of-line boundary on the Apply kernel, consistent with every other operator
boundary in the thin dispatcher. Recheck the exact consumer-local dispatcher
and library dispatcher, all ordinary operator tail blocks, plus callers after
the change; a source attribute or isolated library-frame match is insufficient.
No build or timing was run by this reviewer. Parent independently confirmed
the same inlined Apply cause and notified the writer.

## Additional native scope source argument

Native COUNT(DISTINCT *) uses GroupDomain<()>: its visible method delegates to
the unchanged blank_scope::visible_columns. Native eval_aggregate retains the
same distinct-only visibility computation, checkpoint per input, borrowed row
identity when no hidden column exists, owned projected identity when one does,
and same seen-table insertion/skip/survivor fold. The changed expression is the
statically selected domain.visible(schema), replacing the direct helper call;
no contextual declared-domain scan executes in the unit specialization. Native
direct-dispatch aggregate section0085 contains the actual visible_columns call.
This source argument covers hidden shared-blank schemas, whereas the expanded
probe's IRI-only dataset does not. Final native emission must still bind the
static method erasure and original helper boundary.

Ordinary SHACL cost is reviewed separately from ordinary request substitutions.
prebinding::check_pattern_node adds only Apply rejection; old native arms are
unchanged. substitute::for_each_expression_mut is now a reexport of the generic
algebra walk home, whose initial Vec collect/reverse and pop/visit/reverse-child
push sequence and all operand match arms are exactly the original first-party
body. No adapter callback, second traversal or queue was added. Its actual
substitution monomorphization remains a final codegen scope because its home
crosses a crate boundary. enter_push/enter_substitution and own_expressions add
only Apply cases; old native rewrite/leaf-local binding work is unchanged.

Owned clone/drop, traits, retained sizing, node walks, property-function planning
and governor diff inspection finds Apply-specific arms, with old native arm
bodies/work-list ordering preserved. clone_tree::<false> statically excludes
the new EXISTS mapper branch; its function-item callback is zero-sized and never
invoked by ordinary Clone. Appended Leaf::VarPairs keeps old explicit leaf tag
numbers. Source equivalence covers old-arm algorithm/allocation work; dispatch
layout/codegen and final emitted mapper erasure still need final attribution.

Further direct-dispatch emission checks: false clone_tree section0023 has only
output + NodeRef tag/pointer arguments, no callback argument or mapper call;
six saves/sub3512/CFA3568 versus base0018 six saves/sub3528/CFA3584. Inspected
operator reservations (base -> candidate) are Extend2696->1224,
Filter2408->1112, Minus1032->968, Union48->48, Lateral1560->1432,
Correlation264->264, Join1128->1128, Slice504->504, Project728->696,
Graph1224->1224, Group2520->2104. These bound frame growth in the inspected
homes, not every consumer specialization or instruction path. Shared sequence,
Graph append and application merge helpers introduce no standalone helper calls
in those native bodies; original iterator/parallel callback boundaries remain.

Native Group's sort calls carry contextual_group_rows/ContextualLane names in
this emitted module. This alone is not contextual work: actual group construction
and drop retain Vec<usize>. The full insertion_sort_shift_left body at base eval
LLVM390522 versus candidate448971 compares exactly (diff exit0) after omitting
only definition headers and numeric LLVM metadata attachments. Calls, branches,
loads, offsets, memcpy sizes and all body instructions were retained. Both use
stride72, ordinal offset40 and key40/Vec24 moves. Thus this reused sort body is
attributably equivalent backend work despite its retained contextual symbol
spelling. Other sort paths and final source binding remain in the writer's full
paired argument; no name-based erasure claim is substituted for reading bodies.

## Apply boundary repair: current main-frame recheck

The candidate extended build has terminal release finish. Before its inventory
was prepared, read its actual existing output main module:
`/opt/.cargo/slots/9cc3b70f2cc3d6cc/0/build/release/build/qualification-454-native-cost/9478a740c725d35d/out/qualification_454_native_cost.qualification_454_native_cost.258b1530d971e06d-cgu.0.rcgu.ll`
and its paired `.s`. This completed attempt is now preserved in
`raw/native-cost-candidate-863c75-thin-apply-ir`, with full dispatcher main
section0170, library section0052 and main caller section0177 under
`raw/native-cost-candidate-863c75-thin-apply-sections`. The matching expanded
BASE lives under `raw/native-cost-base-863c75-extended-{ir,sections}`: main
dispatcher0122, library0056 and main caller0125. Those preserved paths are the
durable attribution for this review, replacing reliance on mutable OUT files.
Native eval_node LLVM42960 now opens with only the original three96/three32
Lift payload allocas, direct pattern/context ABI and internal fastcc. Assembly
40420 restores one push/sub144/CFA160, matching the complete original BASE MAIN
section0119. Apply itself restores this frame and tailjumps eval_apply (40670);
native LATERAL restores it and tailjumps unit eval_application. This concretely
clears the inspected main Apply-body frame leak. Writer was asked to preserve
these exact outputs before another build overwrites them. This observation is
bound to that completed extended attempt, not asserted as final all-scope PASS.
Final library/caller/all ordinary arms, wasm and source-bound evidence remain
required; overall qualification therefore still remains NOT MET pending proof.

Preserved final-native pair independently reread: both main dispatcher frames
are one save/sub144/CFA160, both library dispatcher frames two saves/sub312/
CFA336, and both main caller frames four saves/sub56/CFA96. Ordinary main and
library Graph/Join/Minus/Filter/Project/Union/Extend/LATERAL arms restore their
respective full frames before tail jumps; old Slice/Values required call paths
remain. No capture-pointer ABI or shared-match closure remains in this caller.
The previously demonstrated concrete dispatcher/caller cost failures are thus
repaired in this pair, not merely promised by source attributes.

Expanded receipt comparison independently exits0: base extended log versus
candidate thin-apply log, covering11 ordinary queries,11 actual request-binding
windows and3 configured MemoryRelation workloads (including native LATERAL).
This retains the earlier limits on shared-blank and SHACL inference.
Current sampled full tracked diff SHA-256:
`7465c1c96c13420c8745c74cf85f39f35fd3892db88b9ede5892e17a8d01d522`;
eval.rs `455e49d4485396e39d5ca2c3c7499a080ec59a477780a9f45a33b4fadeda973c`,
binop.rs `15b4243148a98858e10b58b2f54ce7caf6a175ac9cdff8acc2975a474501994a`.
Writer reports no shipping edits during these paired final-native builds;
wasm paired emitted qualification is still being produced.

## Scope correction and final native scope closure

The user explicitly corrected scope: "wasm is not your concern." Additional
wasm cost/frame qualification is therefore removed from acceptance obligations,
superseding item5 and every earlier pending-wasm statement in this review. No
further wasm cost analysis is performed. The affected wasm compilation receipt
remains separate build evidence, not promoted into native performance proof.

Final thin-apply ordinary clone section0006 has no mapper argument/call and its
six saves/sub3512 compare with extended-base0018 six saves/sub3528. Iterative
release_nodes candidate0008/base0013 retain six saves/sub344. GraphPattern drop
candidate0017/base0011 retain five saves/no stack reservation/CFA48; opening
tag load, assume, subtract10, select and switch are identical for old variants.
New policy destruction is reached only in the appended Apply arm. No new native
allocation or unconditional policy field read precedes old-arm destruction.

The generic moved expression walk has no emitted forwarding definition or
runtime call in the final eval module. Its inline provenance identifies actual
seed_minus_in, join_assignments_in, replace_exists_bodies and
substitute_in_expressions callers. Together with the exact original queue/body
move, this discharges the new-cross-crate-call concern without equating SHACL
with ordinary request substitutions. Separate existing hidden-blank and actual
SHACL integration controls provide semantics evidence; source preservation is
the operation-count argument, not their execution speed.

Native operation argument by scope:

| Scope | Why the native cost work is preserved |
|---|---|
| Parsing/admission/prepare/reuse | Parser false specialization carries no contextual state/checks. Ordinary admission performs the same existing height and structural walks, followed by the existing property-function planner. PreparedQuery fields/cache route remain unchanged; contextual projection storage resides in its distinct wrapper. Expanded receipts retain parse/admit/reuse allocations and retained bytes. |
| Filter/Extend/Project | Unit mode statically calls original out-of-line native kernels. Extracted sequence loops retain checkpoints, linking, parallel decision/harvest, row iteration and cleanup. Inline sequence homes add no calls; borrowed Project input preserves original alias/lifetime/drop scope. Final direct dispatcher calls these kernels without an added delivery frame. |
| Slice/Dedup/Union/Join/Minus | Inactive delivery branches call original native kernels. Dedup false excludes adjacent mode and retains shared-blank projection/hash/order loop. Slice uses original cap and row skip/take. Native Union retains original parallel/serial home; Join retains original hash join; extracted Minus loop is inline and preserves original domain/compatibility checks. No yielded blocks or one-row delivery allocation executes. |
| Graph | Unit graph kernel keeps its original boundary. Same fixed/variable graph traversal, inner evaluation and one native output Vec; inline append home preserves schema union/resize/compatibility/append operations. Both named-graph receipt controls are equal. |
| Group/COUNT DISTINCT star | Unit domain excludes contextual lane scans and passes no domain ABI storage. Original partition/link/fork/aggregate/checkpoint/identity/seen-table operations remain. Exact unchanged hidden-blank visibility home and equivalent emitted shared sort body are attributed above. |
| LATERAL/correlation/EXISTS | Unit ApplicationMode/false correlation exclude declared-input map, callbacks, deferred delivery and retry work. Prefix width remains computed once outside merge loop. Native compatibility rejection/order/substitution/drain remain original. False/false/unit substituted kernel has no consumer ABI and retained base264 frame. Configured MemoryRelation LATERAL and native metered receipts match. |
| Traits/walks/planner/governor/SHACL consumers | New arms are reached only on Apply; old visits, hash leaf tags, payload offsets and queues retain their source operations. Boxed appended policy preserves GraphPattern144 and other recorded layouts. No extra native tree traversal was introduced. Actual clone/drop/moved-walk codegen boundaries are resolved above. Property-function shape/planning and service/remote/governor old arms retain their original work, including configured relation interception. |

This is a structural and source/codegen proof: static unreachable policy work,
unchanged native loop operations/allocation homes, preserved enum layout/old
dispatch, repaired attributable call/frame boundaries, and paired untimed
witnesses. It does not claim identical complete executable text or measured
CPU time. Changed contextual-only code size and appended dispatch entries do
not themselves identify an extra operation on a native old-variant execution
path. No known concrete native cost regression remains in this inspected pair.
Final source identity must remain bound to the full hash set below; any shipping
repair after this capture needs an affected-scope recheck before final approval.

## Final native source and probe identity

The following SHA-256 hashes were independently captured from the frozen candidate source; the full tracked diff hash is 7465c1c9 recorded above. They include all shipping changed files and the new compiler, plus both actual expanded probe source/manifests. Compiler identity is rustc 1.100.0-nightly commit 4b6d04e706108ccfeafe2547fbe857dfe8972bad; LLVM 23.1.1. Effective profiles/targets are specified above and paired build logs preserve the full resolved flags.

```text
fba8f39004e426559b0f56e44f507b1ffc2f62d7747303d6c8affd69a94cb136  crates/purrdf/src/reasoning.rs
73a3e0165d12023087c2cbe8eb4a9f97eb5964f3a5c0b45438ccfc967d09a2eb  crates/rdf/src/projections/dataset_description.rs
aa09efc1c43c15aedbfe1a7732eaaa6d57232ab6d5e901ad8bbe743dd25c2f1a  crates/shapes/src/prebinding.rs
de102b2e97e4734683ab5249d5a18a2aed0d5c8d9b4ebb3aa48114d3a41812ed  crates/shapes/src/srl/reads.rs
b7d43faf74556562faf4b3eb166ee00073b1df1eaec14452b157e183bbf4acaf  crates/slice/src/ownership.rs
8f62b06450577af57d558b28fa93e5332439b8f47924e5987a41cb74f28a467e  crates/sparql-algebra/src/algebra.rs
3e05d4af72b01f1a78cdbc41e8f03fbcc1292a3ba9da34676e29a34c745eddd5  crates/sparql-algebra/src/owned.rs
d2bbb8db903546272390232ca3c5d3c1ce1a30e3f3d0ae0d5ea032ee11fd20c3  crates/sparql-algebra/src/parser.rs
907e547db5206d58cee68bb21944c70214d6376efd6079eed6793bab876cabde  crates/sparql-algebra/src/parser/machine.rs
e8af5b2305c8f764495b2cc7036f0f2581ecea612c87bfdf6823677a167b4dee  crates/sparql-algebra/src/parser/triples.rs
dc0558b655bf024ae29566a0e23f7e7be0063626e5368d23883a5a0a219c09f7  crates/sparql-algebra/src/retained_size.rs
1f24f2818fb514e937911ac7be4033a51c13745046e08e51e70b8210e4dd164c  crates/sparql-algebra/src/serialize.rs
c472c2f18c755a9cc78f31ebc7c02aa2da2ba8da1b7b0ab6ba7e0ea5e3d00cbb  crates/sparql-algebra/src/traits.rs
f3f776115b10980139d9cac9d6b02b83d07e30832416217f3cd75dfce4e6efb6  crates/sparql-algebra/src/validate.rs
0be7f62613ef6c246f02cc4a3b1da571ab4f45fc64d64dabfb0c329ce157c657  crates/sparql-algebra/src/walk.rs
5d10a052c6b2c48960d82a113399c6bcfad613589f4ebb2be35c07b407e8ee5a  crates/sparql-eval/src/basic_profile.rs
b28e7d669390753f7623a60104b8120a3c1cac8c2ee3b5ee8979fe0c2bfc9958  crates/sparql-eval/src/bgp.rs
15b4243148a98858e10b58b2f54ce7caf6a175ac9cdff8acc2975a474501994a  crates/sparql-eval/src/binop.rs
ac8c76639beac8f99453b171dfaf7a04fab695286b3a2d7d8941141b3a3e9f87  crates/sparql-eval/src/blank_scope.rs
a05a28885515ca565453a4a9a35d36537250b0b2f018f24255c20ce603fb3fa2  crates/sparql-eval/src/cdt_agg.rs
e5ad4f91ba38d4427ecfad1765d5b2f49f71076aac975794df14b14c6ed4753f  crates/sparql-eval/src/construct.rs
83aff139dc7eca18a6dbed1e8b7ae26ae4b992d7a491b9c59437468de840f9e9  crates/sparql-eval/src/engine.rs
455e49d4485396e39d5ca2c3c7499a080ec59a477780a9f45a33b4fadeda973c  crates/sparql-eval/src/eval.rs
580844d88b58cfaea9ead63b8ac70805d2530da6eb58ee3729e1c58860c1fb85  crates/sparql-eval/src/expr.rs
55111759dc29bac696a697d71b7f6cbd6187f9e70680a2d51ed83f96ff8159fb  crates/sparql-eval/src/governor/soundness.rs
67ba46d2725964cd300e525a043195f80f6ce724c8e0126fba2e240ec12d3bc7  crates/sparql-eval/src/lib.rs
75e6e0a82eecc0bb42356927e1a019c175abd575824b1cecf2f2ffdd2d115ad9  crates/sparql-eval/src/modifier.rs
9001c254f5073a09b08c94f2c411df9a261851a1bc0b602839bda3b178a307fe  crates/sparql-eval/src/prebind_memo.rs
3938cb8c872090ea76431dd5ba2933447c7402a631a28419c17d28b417043b48  crates/sparql-eval/src/property_fn_eval.rs
0cf2e0afca5d83922560710b74ba0eaf6ac0798da97758347031c21647716596  crates/sparql-eval/src/property_fn_plan.rs
3d65646b752e35eb501564db690ee5c8e7cdbff2abada40e8fd7a90100cd43f1  crates/sparql-eval/src/remote.rs
d435ed735658e1bfe4b5f964fc719742da9a321227324ce1103293278844d32c  crates/sparql-eval/src/service_endpoints.rs
30d8f3ef4e745b155a4835e4c2a100d41c5ae9c5d89263731e62ff6906de3bd0  crates/sparql-eval/src/substitute.rs
5765ca9ec18e0e8370f70d07435c081ebfe5699195235bf11166812d8bdff40c  crates/sparql-eval/src/rdflib.rs
1ce2702f04fa60a22310b6742938431d53fa3077d8dc9bc35f69f7d48756e7b9  .stage/rdflib-shim-algebra-level-reassignment/raw/native-cost-probe/Cargo.toml
0ab0355be77595ba3acb1a77bcac8274ef332ed2b0070a6dcfe68a31ba0c5bdd  .stage/rdflib-shim-algebra-level-reassignment/raw/native-cost-probe/src/main.rs
ec8499655a926da195c63c752cd2941fea26534e09b6b13101a841b4db2ccd38  .stage/rdflib-shim-algebra-level-reassignment/raw/native-cost-base-ir-probe/Cargo.toml
0ab0355be77595ba3acb1a77bcac8274ef332ed2b0070a6dcfe68a31ba0c5bdd  .stage/rdflib-shim-algebra-level-reassignment/raw/native-cost-base-ir-probe/src/main.rs
```

## Final disposition

**Native structural-cost requirement MET.** No concrete native proof gap remains
in the reviewed changes after the final source/operation/layout/allocation and
attributable caller/operator codegen argument above. This closes native items1-4
and6; item5 was explicitly removed by user scope correction. Historical failed
attempts remain evidence of repairs, not outstanding blockers. This does not
grant all-task semantic/build/commit/PR completion or claim measured speed.

Independently checked all37 paths in
`raw/t1-final-shipping-sha256.log` with sha256sum check: exit0, no mismatches.
Manifest SHA-256:
`b2f23fcb7007ded58bd9f6aff73f65f443fee231397a2861e6deb3dab79f6974`.
Full tracked diff remains
`7465c1c96c13420c8745c74cf85f39f35fd3892db88b9ede5892e17a8d01d522`.
Expanded base and candidate untimed receipts both SHA-256
`e54367e86c2614eddc029273636dfad6bccb6817e7c4988dfacea1f8adf52d32`.
Paired native LLVM artifact identities:

| Home | Base extended | Candidate thin-Apply |
|---|---|---|
| Main LLVM | 3d5b57ebe1a45311f4f96de90e588327fa8f67f3021380ed726d08a4b0667fff | 2957ee6814da794b79be0bdf2c995f4cabaf20f21fb5191e464060de6c93db55 |
| Eval LLVM | d7a239234e188e6f81a8b47d2dd3f555df84f440a7c338c71a6f306808450509 | 5d69206b050410b998df8433335585b0d24673cc2fa5506af77b753cb3dd8ece |

The final verdict is bound to this manifest/artifact pair and its recorded
compiler/effective profile/x86 target identity. A subsequent shipping change
requires only its affected native cost scopes to be rechecked. Reviewer ran
no builds/tests/timings, produced no probe debris, and modified only owned Stage
review reports. No source/base/consumer files or sibling evidence were mutated.
