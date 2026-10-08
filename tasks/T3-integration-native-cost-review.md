<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Integration native cost rebind

Disposition: **MET for primary ordinary native Rust cost at the final static-tail
source manifest c9492b9fb2b85b9ad8f5f0a1669d1a24cf154553f3c94f7a414292c727bab2a8,
against fresh main dfc0c21adabe557e5d11f027aaf2576bddbe1dd6.** Signed Task2
native MET remains bounded to56e636497c049debe2de5445389abf3e018d4783 and its
captured artifacts. Current integration baseline is fresh main
dfc0c21adabe557e5d11f027aaf2576bddbe1dd6, tree
8564b8bd94da23bb073c5dc4e4768fa733d55ad6. Review owns only this report; no
shipping changes, builds/tests, timings, wasm checks, commits or forge actions.
Sole writer resolves and qualifies the actual base synchronization.

## Minimal invalidated seams

Inspecting actual main delta863c75e2c1e13001bcbe025820d257bd0ee49c4f→dfc0c21,
the algebra crate is unchanged. Signed Task2→fresh-main is not the main delta:
it also subtracts the issue's parser/policy/compiler work, so that comparison
must not fabricate an unchanged-proof invalidation.

Main adds DivisionPolicy to QueryOptions and EvalCtx, exact numeric work and
scratch admissions, WorkerDeferral state/ordered commits, scratch snapshotting
and governed parallel row-loop paths. Native modifier/binop/expression/
row_checkpoint and governor bodies have changed. Old absolute EvalCtx/frame/
resource receipts cannot be promoted to this composition. Compare fresh main
with resolved candidate for the following actual affected boundaries:

1. Ordinary QueryOptions/EvalCtx/prepared/layout and native operator dispatch/
   caller frames, direct arguments, tail transfers and unit RowDelivery erasure.
2. Shared native Filter/Extend/Project/group/application/native correlation and
   row-checkpoint paths composed with main's division/exact-deferral/scratch
   changes. Main's own new work is common baseline behavior, not an issue
   regression; verify the issue adds no extra ordinary operation/state/loop.
3. Ordinary Python host query and actual PyO3 caller/capture/drop boundary after
   division keyword propagation, parsing before work and with_division options.
   Unit host mode must still carry no contextual scope/term state or forwarding
   overhead. Correctness of the conflicting integration is separately gated.

Unchanged parser/iterative traits/admission/boxed-policy source proof and prior
qualified source operation arguments remain reusable at matching source hashes.
Do not impose a new full parser/traits corpus or unrelated workspace performance
ceremony. Fresh paired actual evidence is needed only where this changed
composition invalidates old attribution.

## Coordinated affected evidence

Requested sole writer produce sequential fresh-main/resolved-candidate release
native probe main/algebra/eval IR/assembly and identical untimed ordinary
parse/clone/drop/prepare/admission/reuse/request-binding/relation/resource
receipts. Remove the Task2 ApplicationPolicy-layout print on both probe copies:
fresh main has no issue policy type, so that added print cannot be a common
probe. Retain ordinary enum/query/prepared layouts and all original workloads.
Also requested matched release ordinary Python query/PyO3 caller/closure-drop
emission, explicitly retaining effective compiler/profile/target stage identity.
No timing measurements or wasm proof requested. Final source freeze/OID and
paired receipts remain pending while the writer resolves real conflicts.

## Resolved-source initial recheck

Conflict status is now clear. Cached algebra diff against signed Task2 is exactly
empty (0 bytes). Actual parser/machine SHA remains
e5acb6e57811d5ea866403093efceba9a2d2ffbda1db84b5793648632aa4158a and traits
SHA7583787a766ea1a27e8e81b623aaa42faf95abf64c66510d5446ed55ded70430,
matching qualified Task2 source. Reuse the parserfalse/old-node trait/admission
source proof without manufacturing a new parser concern from the main delta.

Read shared host repair at query_impl: division_policy is called before unit/
contextual mode selection, substitutions, relations and snapshotting. Both
query routes receive the same options.with_division(division). Current sampled
host SHAaa348d637fc450621cb347535b5104b8b5cb5d1e0a549941de93ed56b5e6ce71;
eval.rs SHAe9557aefe1a5ebff5cdcd9f4a478768200463ac78f7578c0a5bb78b9db04420a;
modifier.rs SHAfdcc199c69a968f438ee942769144cb4ef639836312b41f95df26390a1fd37b1.
These are preliminary source observations while writer qualification proceeds,
not a substitute for the final freeze and matched actual emitted boundaries.

## Actual merge repair attribution

Writer reports/read source confirms five real modifier merge seams: one native
eval_aggregate call needed its existing generic domain argument; contextual
custom init, custom finish and AVG finish now inherit division/error absorption,
and contextual finish accepts those arguments. Native GroupDomain is still unit,
so the restored argument has zero source state; fresh actual Group/kernel
boundary must bind it. Initial `raw/t3-rust-start-sha256.log` (38 paths,
SHA2e4cc6e6e085ac335c88c59ec56593f628d55ece2a6b5261dd51e835180e27fc)
has one modifier.rs mismatch and is explicitly historical after these repairs.
No gate or emit is promoted across that source change.

Consumer reviewer also found new-main numeric admission/worker-deferral semantics
missing in contextual streaming paths. Those are actively owned repairs. If the
writer factors the existing xsd sum_chain home into an incremental shared state,
review that precise native home delta as well: equal source state/updates and
inline push/finish, with actual native sum_chain/aggregate caller emission where
needed. An O(n²) repeated-prefix scan in the ordinary numeric home would change
its algorithmic cost. This is a conditional
affected seam, not a speculative rejection of source that has not been written.

Actual shared SumChain extraction is now present. It holds the original five
locals (total, running shape, longest whole, largest scale, count); native
sum_chain iterates once and ignores push's returned incremental step. Source
order/state updates and O(n) complexity are preserved. Native emitted proof must
confirm push/finish inline and unused step/struct storage disappear. The native
aggregate_numeric_cost tail extraction also changes a bounded/default early
return into chain.then(tail), with tail=ZERO in that case. Equal Cost values do
not alone prove no redundant max/saturating-add operation. Sent writer the
narrow actual inlined aggregate-cost SUM/AVG caller check; preserve this precise
native source seam in the final argument rather than declare it unchanged.

Writer restored fresh main's bounded/default explicit early return before the
shared tail call. That concrete source-operation concern is repaired; no new
native then(ZERO) fastpath is retained. SumChain's state/update/inline proof is
still the precise new native home seam. Current20-test build source remains
provisional: the consumer reviewer found actual volatile custom aggregate
buffering/step-order risk under numeric admission. Writer owns preserving
volatile updates or the coherent incremental custom-cost seam; no final proof
freeze or cost emission is available yet.

Current direct collaboration sends to parent fail with `agent thread limit
reached` for both writer and this reviewer. This report retains the exact current
status for root readback; that communication failure is not a native PASS or a
delegation of shipping remediation to the user. Reviewer remains read-only on
source and no reviewer process/probe/debris exists.

## Contextual custom-order repair and remaining native seam

Consumer reviewer reports the repaired contextual 21 controls PASS. Read current
ContextualAccumulator::step: Custom appends one accepted tuple, obtains the
caller's complete-fold cost declaration for that prefix, charges work growth
above admitted_work and conservative bytes, then immediately invokes the custom
step on the last tuple. This preserves volatile callback/update and early-error
order. An opaque caller's complete-only declaration may itself scan each prefix;
that cost applies only to explicitly contextual Custom evaluation, not ordinary
native SUM/AVG or the native custom aggregate path. It is not evidence of a
primary native regression. The incremental shared SumChain remains one pass.

The native disposition is still PROOF PENDING solely on the affected fresh-main
rebind: final source identity, paired actual native operators/dispatcher/callers
and host unit boundary, untimed common native probe, and compiled SumChain/
aggregate_numeric_cost/tail erasure at actual native callers. These are concrete
changed seams; unchanged algebra parser/traits evidence remains reused. No paired
final T3 artifact has yet been supplied to this reviewer.

Read newly recorded `raw/t3-final-rust-start-sha256.log`: modifier.rs now
76c868e6ed54bc74860f3b72fbff1fea058719290e00e19da3abfaad27b6ab87;
xsd cost.rs is
0fb10be2d0047d51f37a9a426e034461b95915c9da29a7f5f0ff292c1066b355.
The new common probe copies have identical SHA
0ab0355be77595ba3acb1a77bcac8274ef332ed2b0070a6dcfe68a31ba0c5bdd,
as recorded in `raw/t3-paired-probe-source-sha256.log`. Read the full workload
selection: original eleven queries and matching binding windows remain; bounded
and large arbitrary-precision SUM/AVG queries add two actual native numeric
windows, with exact and fixed half-even AVG policy windows added separately.
The three configured relation queries remain. No clocks are used. These cover
the extracted numeric seam's execution and resource/allocation observations;
they do not replace compiled operation/frame evidence. Inventory selection now
includes aggregate_numeric_cost, aggregate_numeric_tail_cost, sum_chain and
SumChain. Detached baseline receipt names actual dfc0c21 main. This records probe
adequacy before execution, not a PASS receipt.

Writer has refreshed the final paired probe source receipt at
`raw/t3-final-paired-probe-source-sha256.log`; both copies now have SHA
aef409ac8b1122df07bab53f1053f5356decb5b3059db583013e7664bea40587.
Read the full final probe (91 lines): same thirteen query/thirteen binding,
two policy and three relation windows described above, all actual parse/prepare/
execute calls and no clock. Earlier 0ab0355b receipt remains historical. The
source identity used by actual builds must be this final receipt or a subsequent
explicitly reviewed identical-workload update.

Independent read-only sha256sum -c of the current final Rust manifest returns
exit0/all39 paths OK. Manifest SHA
da0cd281ff73bdadd980f28e0d82f9565e88d6e7274b540e686b803642892df2;
final paired-probe receipt SHA
bf87362c394cb99c63f03d37e99bfa5f4db080dbf7aad39637ff53d1fd3dc8b6.
Writer reports the final affected gate is still executing its contextual target,
with earlier targets passing and no proof emits started. This source check is
attribution only; no incomplete gate or unavailable emit is treated as PASS.

Consumer reviewer now independently reports release Rust controls terminal
2306 PASS and post-test source check0. Clippy identified a contextual finish
absorber's missing unit-return semicolon; writer owns the narrow hygiene repair.
It changes no ordinary native body, but final manifest/codegen attribution must
refresh after the repair. The previous da0cd281 manifest check is historical.

Repaired freeze receipt `raw/t3-lint-repaired-rust-start-sha256.log` SHA
cd43f38855986c68eb10c2979267a1659d31081f4247798383bfb0de7909af14;
manifest diff has only modifier.rs, now
fc8f8eae8574082d22df923c97fa15a7391367a31c881c98d3514280dab6675a.
Read the contextual finish closure: absorbed-code call now ends with semicolon.
Writer reports release Clippy requalification active; no proof emission yet.

Latest final freeze `raw/t3-final-lint-repaired-source-sha256.log` SHA
0a599fc5bcc54c8585b2cca7f5daafad3dc3707f3b4217e864c40d9727bc9a95
differs from preceding lint-repaired manifest only in contextual test source
(now5fd7872ac5b8983de918023f784f84b3c24f064eec46e48e4870b39d808f652d).
Shipping hashes remain identical. Writer confirms selected release Clippy
session87963 terminal exit0 (`raw/t3-final-lint-repaired-release-clippy.log`),
quiet current manifest check0 and final common probe aef409ac unchanged.
Contextual test session48048 is still active at the pool-consistency case; no
cost emits started. No native cost disposition is inferred from these gates.

Contextual rerun48048 terminal0/all22 and release Clippy87963 terminal0 now
confirmed by writer. Fresh-main Python emit53519 terminal0; preserved full module
`raw/t3-base-python-ir/purrdf_native.ll` and .s, artifact manifest
`raw/t3-base-python-artifacts-sha256.log`, inventory
`raw/t3-base-python-inventory.log`, full bodies in
`raw/t3-base-python-sections/`. Candidate identical-command emit14010 is active.
Read base section0052 actual ordinary query: internal fastcc original arguments
plus division, 200-byte closure captures (division24bytes; original substitution/
relations/path references offsets168/176/184/192), division_policy invoked before
host work. Actual frame six pushes/sub2312/CFA2368. Base section0029 actual
RdfDataset eval_node: internal fastcc, pattern144/ctx1040/result104; direct old-tag
dispatch arithmetic, native frame one push/sub144/CFA160. These are the fresh
actual matched-stage baselines, not evidence against old T2's smaller absolute
EvalCtx or capture. Candidate comparison remains pending.

## Actual paired Python native-boundary comparison

Candidate emit14010/inventory97289 terminal0; current freeze check0. Full candidate
module `raw/t3-candidate-python-ir/purrdf_native.ll` SHA
f820276cbda145eccd98604fcf29ab45c23e75f18e73a1c29f364e4e1659a0d5;
.s SHA64550348d712e48ed2f9c8946f3a9a2e4c0d325b042716a5cd3e564acf740bc6.
Base LLVM SHA5a4da4638f7c6fc1942a3d480cac401f659b18950f594328c05d809a0b790792;
.s SHA725021feb97d6eeb821c8ed302ce6684b9bdc6ceca3b6e9fd8f0da7788a632aa.
Exact preserved originating output directories are in corresponding
`raw/t3-{base,candidate}-python-emitted-directory.log`.

Read candidate ordinary query full section0065 (LLVM762796), base section0052.
Original13-argument internal-fastcc boundary remains; division24-byte capture and
original reference offsets168/176/184/192 remain. Both captures200, native query
frame six saves/sub2312/CFA2368. No scope tuple, TermValue ownership/drop, unit
selection/dataset call, mode check or forwarding ABI remains. Both invoke policy
parse before substitutions/relations/snapshot. Assembly bodies are **not**
byte-equivalent: stack-slot/register allocation and error cleanup block placement
change. Call inventory differs by relocation of the existing deallocation error
block and renamed prepared/query/drop symbols, not a newly called mode helper.
Do not advertise full instruction identity from equal frame/capture alone.

Actual PyO3 ordinary caller is base LLVM684431/candidate766163 in full modules.
Compared complete function bodies: identical instructions/control-flow/allocas
after exact own impl-symbol and metadata-ID renaming, except five weakened
readonly/captures(none) attributes on the existing query invoke's owned config
and division references. These attributes add no operation; both actual callers
six saves/sub664/CFA720 and directly invoke the same13-argument ordinary query.
No added wrapper/call depth exists. Unit closure drop candidate section0025
(LLVM142023) matches complete base section0018 body (LLVM129549, deduplicated
query_governed closure) exactly after own definition/comment and metadata IDs:
same relation/substitution/config/namespace drops, no contextual selection drop.

Read complete native RdfDataset dispatcher assembly base section0029/candidate
section0037 (LLVM289671). Opening load/sub10/cmovae/jump-table sequence identical;
same three-argument internal-fastcc ABI, pattern144/ctx1040/result104, native frame
one save/sub144/CFA160. M::ACTIVE/unit dispatch/consumer storage are eliminated.
Every old operator arm retains its loads, option-validity check and original tail
transfer or call/restore sequence. Slice's unchanged block moves in layout;
Dedup/Graph/Group/Lateral targets rename to their specialized shared homes;
new Apply block is reachable only by the new tag. No additional ordinary arm
load/check/call or recursive live frame is present. Native probe/library and
numeric-cost emitted comparison remain pending, so overall disposition remains
PROOF PENDING.

Paired actual verbose Python rustc commands both use the managed
nightly-2026-09-14-x86_64-unknown-linux-gnu compiler, edition2024, opt-level3,
codegen-units1, strip=symbols, target-cpu=native, -Dwarnings and
--emit=llvm-ir,asm. Neither has -Clto. Preserved modules contain no ThinLTO,
EnableSplitLTO or LTOPostLink flag. Compare matched emitted optimization stages;
do not infer final-link LTO repair or claim these artifacts are native-probe
ThinLTO output. Their unchanged external ABI/unit erasure/frame observations
are actual at this matched stage.

## Concrete native numeric extraction failure — NOT MET

Both actual native probe emits/run terminal0; independent complete stdout cmp0
(32lines), writer supplied identical SHA
d3e5f397adb54914d7414616748ce3d905de8fd975371ef643462a3d1feda683.
This proves equal sampled layouts/allocations/retained/peak/resource records,
including all numeric windows, but does not prove CPU operation equivalence.

Read actual candidate aggregate_numeric_cost full section0245 from preserved
`raw/t3-candidate-native-ir/qualification_454_native_cost.purrdf_sparql_eval-0d067d9d4a72ce33.purrdf_sparql_eval.5766da4c1a18e898-cgu.0.rcgu.o.rcgu.ll:876814`.
LLVM body still contains %8=alloca[80xi8] SumChain state. Section line152 calls
internal-fastcc SumChain::push for each successfully shaped operand (ASM717).
Section line488 calls aggregate_numeric_tail_cost (ASM745). The exact full
section SHA is4eda111dae58f102baf22bd9758cc9b702429c33a7ee43236f221e91b506cee9.

Fresh main full section0117 comes from
`raw/t3-base-native-ir/qualification_454_native_cost.purrdf_sparql_eval-f1be5881d2e83378.purrdf_sparql_eval.5af75fb13b780118-cgu.0.rcgu.o.rcgu.ll:790163`
and SHA f50bf04c1a32a5e10e160c323b59a79d32b4c96c60c1207e12f892fc6f5f27e4.
It contains the one-loop count/max/shape/cost recurrence and final tail directly,
with no such helper calls. Native caller frame baseline264/CFA320 versus
candidate248/CFA304 is smaller in isolation, but candidate push section0239
has six callee saves plus one8-byte slot/CFA64 and reads/writes state fields,
and tail section0246 has four saves/sub152/CFA192. Those callees execute while
the caller frame stays live; neither added call/frame/storage is erased.
The out-of-line tail also repeats total-is-bounded/default-policy loads,
comparisons and branch after the native caller already ruled that early-return
condition out. Its opening IR lines8–17 make this additional work concrete.

Source #[inline] annotations were insufficient at the actual ThinLTO stage.
This is a concrete NOT MET seam, not an inference from sampled allocations.
Writer notified to enforce targeted helper erasure at the native cost caller,
preserve this failed artifact pair and qualify fresh candidate source/codegen/
untimed receipts. Root direct-send still fails thread-limit; this report is the
durable immediately visible finding. Reviewer makes no shipping modification.

Writer accepts the concrete blocker and preserves failed original candidate
artifacts. Targeted repair changes only push/tail inline attributes to always,
with tightly scoped lint reason comments. Read actual source annotations; no
arithmetic body/ordering change. New freeze
`raw/t3-inline-cost-repaired-source-sha256.log` SHA
f8bfa91de3a3ab7d556595c04316ce1b965f154e96e0f01bb48c928c46f4d8cd;
manifest diff only modifier751ceaf699dcc9b3b547cd9bd15ed4f0ce97e022a78d7c7e5d051e49b406a178
and xsd cost6228ad322df4ceb8596067a9eae705e84dc85f4b252bd07b5727daa658d85d0d.
Candidate native re-emission5357 active, no acceptance inferred from attributes.

Independently read original paired actual native main caller base section0144/
candidate0277 full assembly: all instructions/control transfers match after
symbol/LLVM/EH/function label IDs; both four saves/sub56/CFA96. Ordinary path
restores complete frame then directly jumps to eval_node; guarded path retains
the existing EvalScopeGuard and direct call, no dispatcher capture or forwarding
frame. Main dispatcher base0141/candidate0274 same one-save/sub144/CFA160;
same original three96/three32-byte IR allocas and pattern144/ctx1040 ABI.
Library dispatcher base0056/candidate0144 also remains separately attributable;
the final repaired native pair must retain these already demonstrated boundaries.

## First forced-inline recheck — calls repaired, one concrete extra check remains

Repaired emit5357/inventory41482 terminal0, new complete artifacts under
`raw/t3-inline-cost-candidate-native-ir/` and sections, stdout again cmp0 with
fresh main. Actual aggregate_numeric_cost section0244 (same candidate eval module,
LLVM876930) now has no SumChain::push or aggregate tail call. The state alloca
and return traffic disappear; original eight allocas and six-save/sub264/CFA320
caller are restored. Its single-pass recurrence has the original scalar state/
updates. This resolves the added helper call/live-frame/state issue.

**NOT MET remains**: native already-excluded policy check survives even inlined.
Section0244 original bounded/default block361 performs %362–368 and early return.
The new additional block369 then computes %370=and policy,-4294902015,
%371=eq77309411328 and branch before the SUM/AVG selection. Fresh main0117 has
no second policy check. Reached for bounded total with nondefault native policy,
including exact/fixed policies in the actual paired probe. Assembly lines1203+
contain the additional movabs constants, mask/comparison and branch; this is
not merely unused LLVM metadata. Writer notified to eliminate that native
redundant test while preserving the contextual fast path. No acceptance from
the inline annotations or repeated cmp0 receipts.

Read concrete static repair: shared tail<const CHECK_FAST_PATH> keeps its check
only when true; contextual finish calls true, ordinary native calls false after
its original early return. Same shared arithmetic body, no runtime mode state
or alternate numeric implementation. This removes the redundant native test
structurally. Final actual emitted recheck remains required; prior forced-inline
mask artifacts retain historical NOT MET evidence.

## Actual static-tail repair proof

Final static-tail freeze SHA
c9492b9fb2b85b9ad8f5f0a1669d1a24cf154553f3c94f7a414292c727bab2a8
(`raw/t3-static-tail-repaired-source-sha256.log`); only modifier changes since
f8bfa91d, now815f752eda5df480766e5d1d7f5dcbfd04de257480f273209ade5e7ae856c7ac.
Repaired native emit53422/run/inventory84710 terminal0; source check0.
Actual full preserved module `raw/t3-static-tail-candidate-native-ir/qualification_454_native_cost.purrdf_sparql_eval-0d067d9d4a72ce33.purrdf_sparql_eval.5766da4c1a18e898-cgu.0.rcgu.o.rcgu.ll`
SHA9a948c3eef5dedab0a0cb5499800cd3a0b0061aa9892754a30bd40cc67086b77,
.s SHAe139ce4cc5b611ba76422c7c58779af0fb868cbab03594777fd2f7d56005c7f1;
full-section0244 SHAfbde684869597a65018e97b4691f6cfe314427a3cc0e0ecf183ec6fcfeef9182.
Matching baseline full eval module SHA
b124e95c9abae6a4bf090329a90bebb274dcbb48293f3b4ad5a1b23bdbc72a41,
.s SHA1d2173bf6a6eb317b812104369d88c17711c10fb91efe4b3946860bdb2a6c727.
Complete main/algebra/xsd/binary identities stay in the corresponding
`raw/t3-{base,static-tail-candidate}-native-artifacts-sha256.log`; final candidate
executed binary SHA94b41a5b7c36da8b39ff1438ed176c3e4d8030b336f98f60fa7588d166885400.

Read final actual section0244: same eight native temporaries and264/CFA320 frame,
no helper call or live SumChain storage. Numeric per-operand loop still holds
the original count/running-shape/maxwhole/maxscale/cost scalar recurrences,
one shape conversion per survivor, identical count/max/carry/bounded/add-cost
decisions and one loop traversal. Push's unused return is absent. Native
block361 now goes directly to SUM/AVG selection369 after original early return,
exactly matching main's path; both masked duplicate check and its movabs/and/
compare/branch assembly sequence are gone. The helper constfalse adds no ABI
state or runtime test. Tail remains inlined; same render/division/quotient
operations and number of Cost combinations per SUM/AVG path, reassociated using
associative nonnegative saturating-add/max. No new shape conversion, allocation,
loop, or operator callback exists. Its complete IR has fewer static sat-add/max
sites (27/6 versus28/7) from joining shared tail paths, and actual assembly still
has the same20 calls; these counts corroborate the source/control-flow argument,
not a numerical allowance or a substitute for reading the affected paths.
Both concrete native numeric blockers are resolved at this source/artifact pair.

Independent full final untimed stdout cmp0 again (same d3e5f397 SHA) covers
thirteen query/thirteen request-binding/two division-policy/three configured
relation windows plus original layouts. No time measurements. Core values and
resource/allocation costs match fresh main, including exact/fixed and large SUM/
AVG, all parse/clone/drop/prepare/admit/reuse windows. This is separate from CPU
operation and frame proof above. Release Clippy96157 terminal0 confirmed by
writer; final same-command host emit46295 is active for final-source binding.

## Integrated native source and operator argument

Final candidate's ordinary QueryOptions and EvalCtx field sets remain exactly
fresh main's DivisionPolicy/WorkerDeferral/governor/scratch state. Contextual
delivery, GroupDomain, graph-selection flags and local trip cells add no member
to those ordinary structures. False/unit specializations bypass contextual
compiler/delivery/streaming state. Fresh main's ordinary row-checkpoint order,
worker deferral/ordered commits, expression barriers, division propagation and
exact admission calls remain in the shared ordinary bodies. No per-native-row
scope walk, contextual mapping clone, consumer callback or compatibility graph
restriction check is introduced.

The fresh-main shared Filter/Extend/Project sequence bodies retain their native
child admission/lift/link/schema/row/commit actions and always inline into the
original native kernels. Actual candidate Extend0145 contains the existing
parallel init/commit calls with renamed sequence closure identities, no call to
an extra sequence boundary. Native unit dispatch routes directly to original
Filter/Extend/Project kernels. Group's added generic domain is unit: visibility
still invokes the unchanged blank_scope::visible_columns once at the existing
DISTINCT-star site. Contextual domains do not add native projection mapping or
hidden-blank work. Native LATERAL's width remains cached once before merge loop;
shared application unit retains the existing driver/interception/correlation
substitution path. Apply metadata/runtime refusal is reached only by Apply;
ordinary native parser/admission remains strict.

Actual final native kernel sections (baseline→candidate) corroborate source:
Extend0057→0145 frame2984→1688; Filter0058→0146 2744→1688;
Group0068→0168 2984→2488; Graph0067→0166 unchanged1224;
Dedup0066→0163 unchanged744; LATERAL0061→unit Application0152 1560→1432.
No larger operator frame is introduced. This is not a blanket full-assembly
identity assertion; source operation/algorithm equivalence plus compiled erased
mode/helper ABI and actual frames supply the argument. Generic native minus/join/
union wrappers are false/unit and preserve original evaluator bodies; native
correlation and EvalCtx guard operations carry fresh main's existing state.

Algebra/parser/owned traits/clone/drop/hash/walk/retained storage/admission/planner
and ordinary substitution source qualifiers remain inherited from signed T2:
fresh main changed no algebra, and this integration added no shipping algebra
delta against signed T2. Original tags/layout storage remain append-preserved;
Apply boxed policy/new trait leaf and clone/planner metadata traffic remain
Apply-only. Native request substitutions/PF/reuse and shared native Graph/DISTINCT
star blank visibility have unchanged first-party source homes and fresh paired
receipts. Untimed IRI probe does not itself claim a concrete blank/SHACL witness;
those source-qualified paths and separately owned Rust controls supply that
coverage. Reuse the existing signed T2 exact traits/false parser/drop/admission
proof honestly, with fresh source equality, rather than rebuilding it to meet
an artificial review count.

Actual native-probe command uses managed nightly2026-09-14 rustc1.100.0-nightly
(4b6d04e70,2026-09-13), opt3, explicit lto=thin, codegen-units1, strip=symbols,
target-cpu=native/znver5, -Dwarnings, edition2024 and LLVM/assembly emission.
Matched actual target triple x86_64-unknown-linux-gnu. This explicitly differs
from the Python matched no-LTO emitted stage; do not rename either as a different
optimization stage or infer an unseen production-link repair. Full paired logs,
source hashes, complete emitted module and executed binary hashes are retained.
No elapsed-time performance claim, numerical allowance, wasm hold or dependency/
feature/copy-based implementation is part of this argument.

## Final host rebind and disposition

Final writer host emission46295 and inventory16765 terminated0. Independently
verified both preserved complete final emitted files with their hash manifest,
and independently compared full LLVM and full assembly against the previously
reviewed integration candidate: both cmp exit0. The entire files are identical,
not merely selected frames, signatures, call counts or normalized text. Thus the
already-read host query, actual PyO3 production caller, unit capture/drop and
ordinary native dispatch arguments above apply exactly to this final emission.
The numeric repair lives in its separately emitted evaluator implementation;
the matched no-LTO Python stage contains unchanged host machine code, while the
final paired native ThinLTO artifacts discharge the actual numeric-body cost.
Neither optimization stage is substituted for the other.

Exact final files:
`raw/t3-static-tail-candidate-python-ir/purrdf_native.ll`, SHA
f820276cbda145eccd98604fcf29ab45c23e75f18e73a1c29f364e4e1659a0d5;
`raw/t3-static-tail-candidate-python-ir/purrdf_native.s`, SHA
64550348d712e48ed2f9c8946f3a9a2e4c0d325b042716a5cd3e564acf740bc6.
Their full manifest is `raw/t3-static-tail-candidate-python-artifacts-sha256.log`;
emission log is `raw/t3-static-tail-candidate-python-codegen.log` and actual
output directory is `/opt/.cargo/slots/10d1255bdd2ffab0/0/build/release/build/purrdf-python/6a4e6e189ef185e6/out`.
Final source39-path manifest
`raw/t3-static-tail-repaired-source-sha256.log` independently checks quietly0
again after emission, with full manifest SHA
c9492b9fb2b85b9ad8f5f0a1669d1a24cf154553f3c94f7a414292c727bab2a8.
Matched command/compiler/profile/target identities are specified above and
retained in full verbose logs; no unseen linker/internalization claim is needed.

Final native library dispatcher baseline section0056 (LLVM252471) versus final
candidate0144 (LLVM278598) retains the original internal fastcc three arguments,
GraphPattern144/EvalCtx1040/result104, three96-byte temporaries and two register
saves/sub312/CFA336. The additional Apply arm has its own unwind personality;
it does not enlarge the frame or add work on old ordinary arms. Combined with
actual main dispatcher0272 and restored caller0275 evidence above, both consumer
and library boundaries preserve the ordinary recursive frame/call route.

Primary native cost obligation is MET at this exact composition. The argument
covers original operation/algorithm paths, retained layout and allocations,
false/unit erasure, actual operator/caller frames and ABI, plus actual compiled
numeric recurrence/tail removal. Equal allocator samples alone did not qualify
the failed attempts: both concrete extra-call/state and duplicate-policy-check
findings remained NOT MET until repaired compiled bodies were read. There is no
remaining concrete native proof gap in the scoped source composition. The
writer's final installed binding semantic checks remain separately owned and
are not declared passed by this cost review. This is not a timing result, a
claim about unseen profiles/targets, or a wasm cost qualification.

Plaintext artifact reading is complete for this review. Lossless gzip/hash-bound
publication may remove only verified oversized plaintext copies after retaining
all original bytes, manifests and reconstruction instructions. Review produced
no live processes, build outputs, temporary probes or source debris to clean.
Historical failed artifacts and their findings remain preserved.
