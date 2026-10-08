<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 2 focused native cost review

Disposition: **MET for Task2 primary native Rust structural cost**, bound to the
final unit-mode15-file Rust manifest and paired artifacts below. Earlier pending,
failed and turn-release observations are retained as history; they do not
qualify an abandoned candidate. This is a focused cost verdict, not complete
Task2 consumer/semantic acceptance.
Baseline is signed Task1 e72660e8769af222387ecc4fd4f84ea9887e43a1, whose native
cost proof against synchronized863c75 is retained in T1-native-cost-review.md.
This review reuses unaffected qualified evidence and reviews only the new host
scope/runtime-refusal delta. No timings, builds, tests, source mutations, commits,
forge actions or additional wasm review are performed by this reviewer. Only
this owned Stage report is written; no probes/debris are created.

## Initial source observations

Initial tracked delta SHA-256
`37dd62f37c2f4233847b9dca203b957453cc0be3631b93975b685a0bc206df96`.
Sampled files: algebra.rs
`42a1c2a9a7c627d211ae79ec5cd3b4d275195096fea5048354a4f3d12b1eafee`,
binop.rs `5bff8e1ac3c8c9873e9c3569ad5cfce0e0d506e8a650548f3783806f28fa22c4`,
rdflib.rs `7aae77ffb009f7e73b7765136097eb45c444d8d59717ef4836730d323565c9d0`,
Python quad_store.rs
`1ea983992a0fb0cd5173fbc84e7ff6c2861c573cc96c215b2cedd36072089d7f`.
The writer remains active; these identify preliminary observations, not final
proof artifacts or source freeze.

ApplicationPolicy adds dataset_required inside the existing boxed policy.
Compiler<const SINGLE_GRAPH> adds no runtime compiler field. Its true Graph arm
builds an existing Apply around the Graph; false compiler excludes this arm.
Reached Apply checks the flag in the existing noinline kernel; the active
delivery wrapper delegates refusal to that same kernel. Native old-variant
dispatcher arms, QueryOptions, EvalCtx and ordinary prepare/execute bodies are
unchanged. build_probes changes crate visibility only. Clone/rebuild/property
planner additions copy the flag only on Apply branches. The shared policy size,
alignment and ordinary enum/layout receipts still need actual final attribution.

The ordinary Python entry now forwards to query_impl::<false> with a scope
tuple. Nested settled/detach closures contain selected_graph and scope captures;
static false branches establish source unreachability of extraction/filtering,
but do not alone prove no extra capture/storage/call/frame in the compiled
ordinary entry. Final emitted false helper/caller proof is required at this seam.

## Concrete identity omission sent to writer

At initial inspection GraphPattern traits::Script Apply's policy field list
omits dataset_required while including every other flag. GraphPattern uses
scripted iterative Eq/Hash/Debug, so derived ApplicationPolicy traits alone do
not preserve the refusal flag in these GraphPattern traits. Sent exact repair
to writer and parent: add the flag's Bool leaf and focused policy identity
controls. Native old arms should remain unchanged; final shared-traits emission
must bind the repair. This is an actual identity invariant omission, not a
speculative native cost regression.

## Required affected final evidence

1. Freeze exact delta/source/probe/compiler/target/profile identities after the
   flag-traits repair and host fixes; no stale Task1 result is promoted to new code.
2. Bind boxed policy and ordinary Query/Prepared/EvalCtx/GraphPattern layouts,
   storage and allocation evidence. Native ordinary parse/prepare/clone/drop
   operations remain source-equivalent; altered shared emission needs recheck.
3. Verify native main/library dispatcher and actual callers retain Task1 direct
   ABI, frame/tail boundaries with refusal still confined to Apply kernels.
4. Inspect compiled ordinary Python false helper/caller for scope capture,
   selected_graph storage, tuple argument and forwarding/frame overhead. If
   residual work is concrete, repair and re-review rather than assume erasure.
5. Reuse unaffected original kernel algorithms/loop/graph/blank/SHACL/native
   correlation proof honestly; inspect touched shared clone/traits/planner seams.

No final native cost verdict is granted yet. The writer owns producing relevant
actual receipts; this reviewer owns inspecting and closing the affected proof.

## Flag identity repair source recheck

Writer added dataset_required as the first Bool leaf in the shared Apply policy
Script and focused GraphPattern Eq/Hash/Debug/clone controls in the existing
rdflib_contextual Rust integration home. The source omission is repaired;
execution and final artifact identity remain pending. Rechecked constructor and
transformation homes: owned clone, expression substitution, property planner
copy the flag; row_operation/apply transformations retain the original boxed
policy and their row-pipeline fusion cannot consume a validated refusal marker.
The marker validator requires empty driver/Graph RHS and forbids every other
policy mode. Retained policy storage uses size_of<ApplicationPolicy>, so its
accounting follows the actual type rather than a stale literal. No further
missing policy transformation was found in this affected source inspection.

## Unaffected Task1 proof reused

Signed Task1 diff comparison shows no new changes to eval.rs dispatcher,
modifier.rs ordinary operators, algebra parser/machine or retained_size.rs.
The new primary engine delta is confined to explicitly contextual preparation
and the new contextual host entry. ApplicationPolicy remains behind the same
Box; GraphPattern/Query/PreparedQuery/EvalCtx source fields are unchanged. The
new Bool uses the existing Leaf::Bool/Tok machinery and adds a token only in
the Apply Script arm; it introduces no new shared work-list variant/layout.
The primary parser cannot emit Apply and ordinary algebra admission rejects it.
Thus native old-variant parse, prepare, reuse, operator/correlation/Graph/group/
blank/SHACL algorithms have no new source operation, traversal or allocation.
Their qualified Task1 argument is reused rather than falsely rebuilt from new
compatibility-only tests. Actual final compile/ABI/capture attribution remains
required for the changed shared/type/host seams listed above.

Writer provisioned read-only signed baseline
`/home/paudley/Active/purrdf/.worktrees/qual-454-task2-signed-baseline`.
Reviewer independently verified HEAD
`e72660e8769af222387ecc4fd4f84ea9887e43a1`, clean tracked status and empty
tracked diff (SHA-256 e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855).
Planned paired evidence is actual release Python LLVM/assembly plus the existing
untimed native probe with identical ApplicationPolicy layout additions on both
sides. It will be produced sequentially by the sole writer after shipping Rust
freeze; no prospective result is treated as executed proof.

Detected the writer's frozen Rust manifest
`raw/t2-frozen-rust-start-sha256.log`; independent sha256 check exits0 with no
mismatches. Manifest contains13 paths, SHA-256
`1a4540aa24d362b389968a7a79970921590459817e6cc2c3646e3a1a85f42b23`.
Signed baseline tree is `72260a771de773d322543b22ea07f1c48cacbf7b`.
The repaired traits source hash is
`7583787a766ea1a27e8e81b623aaa42faf95abf64c66510d5446ed55ded70430`.
Ordinary Python quad_store hash remains 1ea98399 recorded above. This freeze
contains the refusal marker and scripted-traits repair; final receipt/compiler
and emitted-source attribution remain pending.

The paired T2 probe sources independently compare byte-identically (cmp exit0),
both SHA-256 `f27b8505eb977a7b8a03f2dc915b88561c9be0d245120e668141ff8ca91dcecb`.
They add ApplicationPolicy size/alignment output to the prior untimed ordinary
parse/clone/drop/prepare/admit/reuse/substitution/relation workloads. No timing
instrumentation was found; source intent is not an executed result.

## Concrete contextual default-Graph scan finding

At the parent's focused request, examined query_impl::<true> on plain Graph
scope (false,None,false) with an all-default snapshot. The current condition
unconditionally constructs MutableDataset and decodes every quad through
surface_of, although both remove/insert predicates are false for every row.
`surface_of` is not lazy: mutable.rs1347 returns a Vec collecting owned s/p/o/g
TermValues over quad, reifier and annotation tables before the loop runs.
This is an unnecessary contextual O(n) materialization/COW/freeze path, not an ordinary
native regression claim. RdfDataset::named_graphs at dataset.rs844 is a borrowed
Box slice iterator and documents completeness for quad-bearing and declared
empty named graphs. Gate the !scope.0 branch with named_graphs().next().is_some()
to skip this work when no named graphs exist; keep union/selected-graph branches.
An empty iterator proves there are no named rows/declarations to suppress.
Builder.rs1573-1594 constructs this metadata as declared names UNION quad.g
UNION reifier.g UNION annotation.g, so the argument covers RDF1.2 overlay rows.
The single_graph compiler flag remains independently !scope.0, preserving lazy
GRAPH refusal even when the original snapshot is reused. Sent this exact
source-grounded recommendation to parent and writer.

Writer reports final consumer execution exposed a contextual nested-EXISTS
compiler defect requiring an owned repair. The first frozen manifest/install
therefore becomes historical after that edit, and paired cost emission waits
for the repaired freeze. No codegen proof is promoted from the earlier source.

Source repair now read: the contextual selection condition gates !scope.0 by
the complete named_graphs slice's first entry, retaining union/selected-graph
branches. Default-only Graph avoids the owned-surface Vec and no-op COW/freeze.
Current quad_store SHA-256
`2a6b3c7a9123bc5a4b3014732ffd01bf9682616876adf6329e052d3be2abd89d`.
Nested EXISTS repair iterates the enclosing contextual mapping, choosing an
available immediate-driver slot or enclosing-context slot before thaw; it is
confined to the contextual compiler (rdflib.rs hash
`37a5643d1df85887996432f6c168fafe292512a8d4a4bdeadd8b2014f7409a13`).
No primary ordinary parser/evaluator/engine algorithm changed. Both are source
repair observations, not executed final qualifications.

Read completed `raw/t2-nested-context-native-tests-repaired.log`: release finish
and14/14 Rust contextual integration tests pass, including the new policy
identity control. This clears the executed scripted-flag identity omission for
that tested source and checks the contextual repair; it does not substitute for
the outstanding paired native/ordinary-host compiled cost argument.

## Turn-release status

Parent reports two targeted inherited-context attempts still fail the concrete
2-versus1 witness and requests this turn/slot released for an independent
root-cause reviewer. No final native cost verdict is granted: source is again
mutable, paired final emission is not yet available, and the ordinary Python
false helper capture/ABI/frame seam remains unqualified. Previous14-test PASS
is bounded evidence for its source/tests, not resolution of that newer witness.
The scripted flag omission and default-only Graph materialization finding have
source repairs; those repairs must bind the eventual final manifest/artifacts.

Reviewer leaves no live build/test process, probe or diagnostic debris; only
this owned Stage report was changed in this turn. No shipping/source/base files,
sibling evidence, commits or forge state were mutated. Parent will resume this
same owned cost review when the core source stabilizes and actual paired
receipts exist. Overall work remains active and the defect has visible owners.

## Stable-source review resumed

At writer/parent direction resumed against15-file
`raw/t2-current-rust-start-sha256.log`; independent sha256 check exits0.
Final current rootFilter-only visibility fix combines retained context and
returned mapping solely for a leading EXISTS filter's expression compilation;
its row-pipeline returned domain/inputs remain separate. Failed inherited-
context/private-projection guesses are absent. No primary ordinary evaluator
algorithm/state changed.

Newly affected ordinary parser scope: machine.rs folds multiple same-group
filters only under const RDFLIB&&filters.len()>1. For false specialization the
source reduces exactly to the existing owned filters Vec and wrapper loop;
no contextual len/reduce/And/extra Vec operation executes. Final emitted false
parser function/frame proof must bind this new delta; it cannot be inherited
silently from Task1's earlier untouched parser evidence.

Current core controls and affected release Clippy complete successfully in
`raw/t2-current-rust-core-controls.log` and
`raw/t2-current-affected-release-clippy.log`. Those are correctness/warning
receipts, not native operation/capture proof. Writer is producing paired signed
baseline Python release codegen, then candidate and untimed probes sequentially.
Review remains active at the final compiled-boundary seams, with no timings,
wasm investigation, shipping changes or reviewer build processes.

## Isolated contextual Filter allocation correction

Independent source review found the isolated Filter branch cloned its complete
compiled.mapping solely to borrow it for expression compilation. The signed
implementation borrowed that map directly. This added a BTreeMap allocation and
entry clones per isolated contextual Filter; it was not an ordinary native
regression. Writer applied the targeted repair: the owned CunionR map is built
only inside (!isolated).then, and expression compilation borrows
visible.as_ref().unwrap_or(&compiled.mapping). Thus isolated compilation retains
the original borrowed map while the leading root-EXISTS branch still owns its
required context union. The 1909-test and release Clippy receipts above predate
this contextual-only allocation repair; the final manifest/gates/artifacts must
rebind it. The active signed-baseline codegen is unaffected.

The repaired contextual compiler file SHA-256 is
`1a2e3dbdd2ab0e7284513ae4bcc4b4eb64be0a63bda2ec07036dd10796d1c005`.
Rechecked shared deltas against signed Task1: APPLICATION=false admission
returns at the existing Apply rejection before reading the new flag. Unit
RowDelivery returns directly to eval_apply before the active flag test. The
noinline Apply kernel alone adds its flag load/branch; old native node arms do
not call that kernel. Owned clone, expression substitution and property planner
copy the Bool solely inside their existing Apply arms. The iterative shared
traits add two tokens solely in that same arm, using existing Bool/Tok storage.
No ordinary operator loop, caller state field, enum variant, traversal count,
or graph/group/correlation algorithm is added in these new source deltas.

## Preserved signed-baseline ordinary Python entry

Read actual final release baseline module
`raw/t2-base-python-ir/purrdf_native.ll` (SHA-256
`b1b793a02913ad2dcd0e1c854e9476e888068331064806d1dea3891a9e8490e0`)
and paired assembly (SHA-256
`f111691eb9a9d10c43c3d330170d6c23acb01ba16f38ed38a3f02a5f626c865b`).
LLVM line663790 defines the actual ordinary PyQuadStore::query as internal
fastcc with its original twelve pointer/scalar arguments and no scope argument.
Its settled capture alloca is176 bytes, storing existing configuration,
self/query and substitutions/relation pointers. Assembly function starts at
line731280, six pushes and sub2296, CFA2352. These are actual signed-baseline
caller/frame observations, not a predicted candidate verdict. Candidate final
ordinary entry/closure/caller must be compared at the same optimization stage.

## Final Rust freeze for paired evidence

Independent sha256 check of the15 paths in
`raw/t2-final-rust-start-sha256.log` exits0. Manifest SHA-256 is
`18da90d6c47d1ae3db66671e7862bff150ce4b4142007adb3d1b3745cd0cbc51`.
The borrowed Filter correction is now requalified by contextual18/18 controls
in `raw/t2-borrowed-filter-contextual-tests.log` and terminal release affected
libraries/selected-target Clippy in `raw/t2-borrowed-filter-release-clippy.log`.
These bind the source repair without claiming performance proof. Writer's
paired candidate Python emission is active in
`raw/t2-candidate-python-codegen.log`; its inventory source is identified by
`raw/t2-ir-inventory-source-sha256.log`, SHA-256
`f4a439ff8663ba4218a332ce605c3480fc525a46448836f577d31134df7cb5a3`.
No candidate emitted-cost or final native verdict is inferred from build start.

The same actual signed-baseline Python module retains the native RdfDataset
dispatcher at LLVM line270379, internal fastcc with direct pattern and ctx
arguments (GraphPattern144, EvalCtx944). Assembly line294631 shows one push,
sub144/CFA160 and direct old-tag dispatch. Its evaluated closure at LLVM525751
uses direct pattern input with only the existing guard32/tree16 allocas and
ordinary tail call to eval_node. Candidate proof must preserve these boundaries
in this real consumer module as well as the separately emitted probe/library.

## Actual paired ordinary Python boundary failure

Candidate final module `raw/t2-candidate-python-ir/purrdf_native.ll`, SHA-256
`028a7ad2ebf8abe0fb0ff48577fcfb9af5dd8607f042c43413a526b9c5621cfc`,
and assembly SHA-256
`ab016192e4a181e145fc20ca9b8ecf057a8a5a18c8b356324ef78f06219cdcb9`
were read directly. Actual ordinary query definition LLVM746810 retains the
original internal fastcc ABI and inlines query_impl false, but its settled
capture is192 bytes against baseline176. It explicitly stores compatibility
scope true/null/false at offsets184/144/185. Its detached closure/result allocas
are280 bytes against the baseline192-byte corresponding closure. Constant false
branches did not erase captured scope/selected-graph state. Actual assembly
query starts at811214, six pushes/sub2616/CFA2672, against six pushes/sub2296/
CFA2352 in the signed baseline. This is concrete additional ordinary native host
storage/stores and increased live frame, not an inference from a timing sample.

Sent exact finding to writer and parent. A zero-sized ordinary mode/associated
state with contextual mode carrying its own scope/graph data can preserve the
single body without capturing compatibility data in the ordinary specialization.
Required repair proof: actual ordinary query capture and detached closure/drop
state, frame, direct ABI and production caller graph must discharge this cost.
No native PASS is granted; the current candidate is recorded failed evidence.

Checked other actual consumer boundaries separately: candidate eval_node
LLVM289479/assembly311794 remains internal fastcc with direct pattern/ctx,
GraphPattern144/EvalCtx944, original six result allocas, one push/sub144/CFA160,
and the same old-tag arithmetic. Candidate evaluated closure LLVM572424 and
assembly629832 retain guard32/tree16 and five pushes/sub48/CFA96; its ordinary
LLVM block directly tail-calls eval_node. Those inspected boundaries match the
signed consumer baseline. The failure is the ordinary host query capture/frame,
not a recurrence of the previously repaired recursive-dispatcher expansion.

## Typed host-mode repair source observation

Writer replaced the raw const-generic scope capture with QueryMode and an
associated Selection. The ordinary mode and selection are both unit; contextual
mode alone contains (bool, Option<TermValue>, bool). This removes scope and term
ownership from ordinary closure types before layout/drop generation. The
ordinary mode's selection and dataset functions are identity Ok operations;
final emission must prove their Result adapters/calls erase as well as removing
the captured storage. Contextual dataset metadata fastpath and selection loop
remain source-equivalent. This is a source repair observation, not a native
cost qualification; the previous manifest is historical after this edit.

## Actual repaired ordinary host proof

The repaired15-path freeze `raw/t2-unit-mode-rust-start-sha256.log` independently
checks0, manifest SHA-256
`b558417ac39046a6ca70ebe02b087cda343961b05de1cc78fc023c6dd0651804`.
Host source SHA-256 is
`0918fd1f8a17d0e4d88504be88937fe4388365d3e2609306e2daaf7233bc55d6`.
Actual repaired module `raw/t2-unit-mode-python-ir/purrdf_native.ll` SHA-256
`1f0c458d50e92bd5d1244c45dd0a51b69f4eec61695db601667fabc77464d13e`,
assembly SHA-256
`7860e6d9d30703ee5392bb679cd95b0df40d2016f8144d4d041cc785229adb5d`.
LLVM761707 retains the original internal fastcc12-argument ABI. Settled capture
is176 and detached capture192, with offsets144–168 holding only original
substitutions/relations pointers; scope stores/Option<TermValue> ownership are
absent. QueryMode selection/dataset emission contains only contextual tuple
definitions/calls; no unit adapter call or test remains in ordinary query.
Assembly828086 has the same six saves, sub2264/CFA2320, below signed baseline
sub2296/CFA2352. The increased live-frame/capture finding is discharged.

Actual PyO3 caller LLVM764890 versus baseline692821 has identical full IR
operations after metadata IDs and implementation-index symbol spelling changes;
the sole remaining diff weakens four call-site alias/readonly annotations,
without changing arguments or adding instructions. Both actual assembly callers
(candidate831544/base762257) have six saves/sub616/CFA672 and directly invoke
the same original12-argument query boundary; no intermediate forwarding call.
Ordinary query's full direct call list adds no graph-selection/mode operation.
Baseline closure drop is deduplicated to query_governed's192-byte closure
(LLVM130674). Repaired unit closure drop LLVM141854 compares exactly after
only comments, numeric metadata and its own symbol name are normalized. Thus
normal and unwind ownership have no new scope/selected-graph destructor or
walk. The single shared ordinary body retains original engine/registry/
materialization operations. Parser false-specialization and paired native
layout/allocation receipts remain open separately; no final native verdict yet.

## Paired ordinary parser and native receipts

Read complete paired false Parser::resume sections:
`raw/t2-base-native-parser-sections/section-0078.txt` (actual baseline algebra
LLVM156402) and `raw/t2-candidate-native-sections/section-0078.txt` (candidate
LLVM156542). Full IR operations/control flow/allocas compare exactly after
only numeric metadata, explicit crate disambiguators (algebra/lex/alloc-probe),
ThinLTO imported symbol suffixes and anonymous constant symbol naming are
normalized. The complete assembly instruction/control-flow text also compares
exactly after those symbol names and nonexecuting EH location-label names;
all actual branch-block ordinals/targets, immediates, offsets and instructions
are retained. Both have six saves/sub1800/CFA1856 and Parser680 ABI. The new
const-false filter fold adds no load, reduction, And node, Vec allocation,
callee, destructor, frame slot or traversal. Original source fold false branch
and complete emitted body jointly establish this; equality of opcode totals
alone is not being used as proof.

Independent cmp of `raw/t2-base-native-probe-run.log` versus
`raw/t2-candidate-native-probe-run.log` exits0. Both27-line logs SHA-256
`7cc55ec0ce0cdff2e1feade336b09b98153b496868cc523c8be02a8e1187dfc2`.
ApplicationPolicy remains88/align8; GraphPattern144/Expression64/Query264/
PreparedQuery368/NodeRef16 unchanged. The11 ordinary query windows,11 actual
request-binding windows and3 configured MemoryRelation windows retain exact
allocation counts/bytes/retained/peak, plan accounting, row counts and metered
resource vectors across parse/clone/drop/prepare/admission/reuse. Both original
untimed probe main files still SHA-256
`f27b8505eb977a7b8a03f2dc915b88561c9be0d245120e668141ff8ca91dcecb`.
No clocks or timing measurements are used. These finite witnesses corroborate
the source/compiled proof; they are not substituted for whole-domain proof.

## Actual paired native dispatcher/caller proof

Native library sections base0142/candidate0126 retain internal fastcc direct
pattern/ctx GraphPattern144/EvalCtx944, three96-byte result allocas, two saves/
sub312/CFA336. Main base0248/candidate0244 retain the direct ABI, original result
allocas and one save/sub144/CFA160. Main caller base0251/candidate0251 retains
guard32/tree16, four saves/sub56/CFA96. Full main dispatcher and caller assembly
compare exactly after crate disambiguators, ThinLTO suffix and local function/
block-label numbering only; no instructions/operands/branch ordinal are removed.
Library assembly's sole remaining difference is two names of panic constants;
actual referenced48-byte text and location payload bytes are identical. Thus
native recursive call depth, direct pattern load, tail transfers, spills and
live frames retain the signed Task1 boundary. The repaired actual Python module
likewise has eval_node at LLVM289479/asm311794 and evaluated closure at
LLVM568329/asm625720 with the qualified original production boundaries.

Exact full native module hashes are retained in
`raw/t2-base-native-ir-sha256.log` and
`raw/t2-candidate-native-ir-sha256.log`. The false-parser algebra pair hashes are
base LLVM9939a122ffbe18cf4c767522c35fe2a43422b210578c5bf8b94301115d9cf47a,
asmc8660c7701c6a3cb27928eec03a52cafd83169dad617dd55b0e1b4ed1b8b3e6c;
candidate LLVMc00346faf9dabd4e77da2b5f5d8f430bdb7b96347c186bcd609858af051785a2,
asm5f6a1cf93a4148eacabf4ee4aa9ee7995d310f29aa0493a34e7c54a868903b8d.
The candidate freeze manifest still independently checks0 after these receipts;
its only change from the earlier native-core freeze is the typed Python host.

## Shared clone/drop/traits and final source argument

Actual false clone_tree frames also match: baseline section0013 versus candidate
section0006 both have six saves/sub3512/CFA3568. Iterative release_nodes base0015
versus candidate0008 has six saves/sub344/CFA400. New policy storage remains88;
the clone/substitution/planner flag copy is contained in the existing Apply arm,
with no new old-node child, callback, walk or allocation. Actual shared
traits::Script::pattern is retained in paired algebra modules (base LLVM112926/
asm110286, candidate LLVM112974/asm110285). Both internal fastcc functions have
the same direct old-tag dispatch, no IR allocas, six saves plus one8-byte push,
CFA64. The two new field/Bool tokens occur only in Apply's scripted branch;
old native fields/tokens/layout remain unchanged. Eq/Hash/Debug therefore add
no ordinary token/hash/drop work. The unit admission path rejects Apply before
the new policy read; primary preparation cannot admit its extra contextual
kernel work. Reuse the qualified signed Task1 operator/blank/Graph/SHACL/
correlation/substitution algorithm argument for unchanged ordinary branches.
No new primary QueryOptions, EvalCtx, prepared/query/enum state or traversal is
introduced; no dependency, Cargo feature or manifest changes occur in this delta.

## Compiler/profile attribution and final disposition

Artifacts themselves identify rustc1.100.0-nightly commit4b6d04e70 (full commit
4b6d04e706108ccfeafe2547fbe857dfe8972bad); retained `raw/base-rustc.txt` supplies
LLVM23.1.1. Both builds use the same dated absolute nightly toolchain and
x86_64-unknown-linux-gnu target. Captured effective commands on both sides use
opt3/codegen-units1/strip=symbols/target-cpu=native; the actual emitted attributes
identify znver5 and the native feature set. Native probe final commands explicitly
use lto=thin, with actual main/algebra/evaluator backend IR/asm preserved. Python
pair uses --emit=llvm-ir,asm with no explicit lto flag or ThinLTO/PostLink module
flag. Its proof compares those same emitted optimization stages and their
actual ABI/caller/frame; no unobserved final-link repair/LTO effect is inferred.
Unit ownership/type erasure independently removes the source state before
closure layout on the ordinary path. The normal installed module's other
profile is correctness evidence only and is not promoted as this cost artifact.

All concrete native cost findings are now repaired and actually rebound: isolated
contextual map clone, default-only contextual graph materialization, and ordinary
scope/term capture/frame leak. The parserfalse/full native emitted boundaries and
identical untimed receipts close the affected proof. No extra ordinary operation,
allocation, layout/state, walk/algorithmic work, call layer, native recursive frame
or spill is left by this delta under the inspected source and actual compiled
boundaries. **Native structural cost MET** at manifestb558417ac39046a6ca70ebe02b087cda343961b05de1cc78fc023c6dd0651804.

No timing measurements, wasm review, reviewer builds/tests, shipping changes,
commits or forge actions were performed. Reviewer owns only this report and
leaves no process/probe/debris. Full raw artifacts and source/receipt identities
remain preserved; oversized LLVM may be stored losslessly as gzip with exact
decompression SHA and byte-cmp verification by the root. That packaging is not
a performance proof or lossy replacement of inspected evidence.
