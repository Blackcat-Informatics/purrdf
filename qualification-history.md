# Current qualification

## Review/CI remediation — 2026-10-23 current state

Latest production head: d0ef5e401, pushed with normal hooks. The independent
GraphQL.js 16.14.0 oracle passes on that source (integration-graphql-oracle.log),
including the eight list probes, value codec, output types/unions, temporal
losses and reverse import. The one-element allOf around a catalog value reference
caused an actual unlocated GraphQL acceptance divergence on member-not-integer;
the plain-reference repair removes that wrapper and its allocation. Property
loss comments now inspect the original property catalog too.

Current native library: all 989 pass in integration-repair-native.log. The same
command rechecked the 13 previously failing non-emitter integration targets:
11 targets pass, including the SHACL-SHACL differential (unchanged exact ledgers),
the reference interpreter/property suite, full expression cases and ontology
surface. The remaining four failures were three product tamper fixtures whose
term tag offset was wrong and one restored-expression body inspection. Their
targeted repairs are now running with the emitter pin updates in
integration-pins-native.log. These are test adaptations, not new public APIs.

The preceding all-integration run at b049fa51c produced 90 completed target
results, 1185 passes and 101 failures across 14 targets. Every failure group is
mapped to its existing root/body/schema-reference inspection or the fixed
GraphQL coercion defect. All allocation/catalog/census/product determinism
controls in that run pass; the current shipping parser/plan/codec inputs to
those controls are unchanged. Do not label that overall failed run qualified.
Projection pins continue to check complete referenced bodies and located losses;
they now also require actual shared references. The recursive projection fixture
checks the published typed refusal and a finite conforming/nonconforming neighbor.

integration-repair-strict-current.log passes Shapes all-target Clippy before the
latest test-only emitter/refusal/spelling pins; those require a focused current
check. Current-head hosted CI, final review and the actual TypeScript compiler
oracle remain pending. The source worktree currently contains only these bounded
test repairs. The earlier state below is historical where it differs.

Published head: b049fa51c, clean and pushed with normal hooks. Origin main d26bc6e64 was merged normally in
4397a2336; the latest fetch shows no further base advancement. Exact hosted
Rust 1.98 MSRV passed on 7c100b92c. CI remains red; merge is not ready.

96b0ba3d3 addresses all three CodeRabbit inline findings and its duplicate
planned-constraint lookup nit. Normal hooks and push passed. Native library
execution at that change passed 977 tests and failed 12; all 65 parser-module
tests and both new original-domain/target regression controls passed. Replies
with this evidence are posted on all three review comments.

Constructor-proof audit: base/current census exports are capacity-base.json and
capacity-current.json. All 173 constructor-source members match exactly; only
core artifact/identity.rs and ir/pack/bits.rs source fingerprints changed.
The actual diff streams the same framing and generalizes the existing varint
sink; no ID constructor, capacity authority or sink changed. The 40 aggregate
constructor bindings were refreshed to source:b4c07f030aea92a9. Authority pins
and sinks remain unchanged. Core's prior 1398 passing controls and frozen
encoding assertions apply to these unchanged source inputs.

Published bounded compatibility repairs in b049fa51c: original-body fixture inspection,
private bundled structural meta-schema admission through the existing depth/
stack-bounded evaluator, reached-only compatibility constraint admission,
schema-import recognition of the emitted shared catalog, CLI diagnostic/model
diff expectations, and the renamed assembly measurement home. These repair
existing callers; no public recursion mode or LFP/GFP solver is added.

review-compat-library.log completed with 988 passes and one failure. All three
native lint failures and the other original-body/admission failures now pass.
The remaining logical schema round-trip introduced an extra allOf wrapper;
the exact helper-reference conjunction repair subsequently passed all 14
schema-import tests (compat-schema-import.log). compat-strict-final.log passed
Shapes/CLI all-target Clippy before the final ordinary-import allocation
adjustment; current-head final qualification remains due for that small change.
Both current-head oracle generators now pass (compat-typescript-fixture.log,
compat-graphql-fixture.log), including exact source verdicts, loss locations and
reverse reconstruction. These are native fixture checks, not independent
TypeScript/GraphQL compiler execution. The locked Node oracle dependencies are
installed successfully (oracle-npm-ci.log); actual oracle runs follow the live
single Cargo command: all Shapes integration targets in
compat-shapes-integrations.log. Current-head hosted CI and final review remain
pending. All three initial inline threads are resolved with tested replies.
No completion/merge is claimed.

The sections below are chronological historical evidence; their live-command,
publication prohibition, old CI and old-head statements are superseded above.

## Current PR publication authority

PR #691 is open at pushed commit 2e42e8792, with normal commit hooks passed.
The user explicitly ordered publication, a regression issue, review and merge,
and forbids expanding PR scope. The backlog goal is paused. Issue #690 contains
the full 41 failing native shapes test inventory. Read publication-scope.md;
the earlier no-partial-PR/no-commit language below is superseded for publication,
not a waiver of verification. Do not implement new solver/mode functionality here.

No earlier Cargo command remains live. The current command is the supported
product_determinism fixture generator for the census-derived 81d0 stage
(publication-product-generate.log). All 1398 core unit tests passed; the same
native command finished with 947 shapes passes/41 failures, so do not report it
as qualified. Current hosted CI run 40023158628 and CodeRabbit review are live.

## Immediate unfinished acceptance — do not commit

Latest completed integration batch: streaming-identity-shacl-native.log, exit 101.
All 18 incremental footprint controls and ten carried-analysis controls pass.
Eleven change_path_alloc controls pass: both bounded expansions and their unchanged
allocation closed forms, all focus-growth/report controls and binding's original
59 allocation pin. Admission now measures 230, below the old 293 pin; its exact
determinism constant has been lowered to 230 and awaits re-execution. This is an
83-call reduction against the preceding 313 result and 63 below the old baseline.
No original cost ceiling was raised.

The new DAG growth guard initially compared adjacent requested-byte ratios and
failed: depth 32 requests 112077 versus 4 * 27597 at depth 8. This assumes an equal
fraction of geometric map capacity in adjacent samples. The actual depth-one cost
is 3553, so depth 32 remains below the linear envelope 32 * 3553. The new guard now
anchors all 1/8/32 measurements to depth one rather than adjacent capacity steps;
this revised guard has not yet executed. Native observed preparation at 1/8/32 is
158/724/2624 allocations, 32218/162714/638161 requested, 8353/38368/145455 retained
and 13033/63388/249419 peak bytes. Expansion is 14/52/176 allocations,
3553/27597/112077 requested, 84/64/64 retained and 1612/2952/8032 peak bytes.
Every expansion is bounded and contains one affected node with an untouched
disconnected target held in the same data graph.

The six existing catalog controls pass. Static-root observations improved to
18 allocations/3106 requested/zero retained/1674 peak, exactly equal at 1/8/32;
the absolute limits are tightened accordingly and await re-execution. Original
domain admission 2/96 and export/clone 6/544 limits still pass.

Static definitions now record relative footprints once. Reference sites compose
these through a shared acyclic path arena; general cycles use the existing SCC
home and bounded conservative path closures. This is read-dependency composition,
not the still-missing recursive conformance solver. Strictly forward discovery
edges prove acyclicity without SCC scratch; small graph buffers stay inline.
Only actual root entry records targets. Requested maps release construction-only
storage before sealing. Computed references retain their full original domain
and mark unrooted reads when necessary.

Shared arena entries with multiple incoming edges memoize (path, data node) in
the existing native path evaluator; cached outputs are deduplicated sets. Ordinary
paths and unshared entries need no memo table allocation. shared-footprint-graphs.log
passes both new native controls: data-reading diamonds at 1/8/32 retain at most
five path entries per depth and execute; a cyclic read component reaches every
expected predecessor while excluding an unrelated edge. These are footprint
composition/kernel controls, not full recursive SHACL acceptance or universal
linear-allocation evidence.

Admission reductions measured so far: 348 -> 320 by avoiding parser/prefix copies
when no function class is interned, then 320 -> 313 by skipping SCC scratch for
edgeless strong-ownership graphs. Binding's extra static-only data-id row was
removed (60 -> 59). Carried-analysis forgery tests now repack the full-model
digest when replacing AST bytes, preserving the actual identity gate; all ten pass.

Source repair also replaces restore-time unreached-function index cloning and a
swallowed OnceLock::set with complete original definitions through the existing
linker. A new adversarial runtime control and missing-path processor-failure
controls are not yet executed. A newly identified admission cost is Identity::refresh
serializing the entire growing identity after every push. The current uncommitted
core change streams the same canonical encoding into SHA-256, using the one LEB128
writer with an Extend sink and inline length storage. identity-streaming-allocation.log
passes: appending a three-byte label/five-byte value costs exactly two allocations,
eight requested/retained/peak bytes, identical with 1 KiB and 1 MiB already retained.
All 43 artifact unit controls pass before extending the independent frozen framing
assertion. The added frozen framing/varint assertions await their runtime execution.

shared-footprint-identity-strict.log exits 0: current core/shapes/validate/Python
all-target Clippy passes with warnings denied. streaming-identity-shacl-native.log
has completed as recorded above. shared-catalog-core-shapes-units.log is now live:
current core and shapes native library controls, including frozen canonical framing,
both Vec/inline varint sinks, private unreached-function domains and missing-path
processor failures. Collect its terminal result before another owned Cargo command.
Current source has no diagnostic probes.

Current production ownership fixes are runtime-qualified: 33 AST codec controls
pass including the two before/after-product domain adversaries
(original-domain-codec-native.log); all 10 planner controls and 13 linker controls
pass. Planner fixture inspection now uses actual prepared references; the kind
pin names Reference and PropertyReference explicitly. change_path_alloc aliases
those kinds to its real node/property fixtures, preserving every existing cost
ceiling. All 91 schema controls pass after the negand proof fix.

Last measured census digest, now stale after further source changes, is
0623858c1b90bbffb71121a88d65594ac12521ee3bc284524f2253fa1614194f.
The shipped a15 constant/5608-byte fixture are stale. Nine census assertions
fail solely on that stale stamp; 19 census controls pass. Do not refresh again
until related footprint/allocation fixes settle. Current strict Clippy must then
be rerun, followed by relevant native products/absolute caps and normal hooks.
No new commit/push, PR or completion is claimed. Full SCC/modes/solver/hosts and
the eighteen paper controls remain outstanding under the original contract.

## Latest demand-lowering state (uncommitted)

Static-root preparation had real unused-domain growth: 1/8 definitions cost
19/42 allocations, 3158/11916 requested bytes and 1798/5320 peak bytes. The
native constant-growth assertion failed at eight before attempting 32
(static-root-preparation-native.log). Do not erase that original evidence.

Lowering now queues definitions on the shared WorkList, once by native body
identity. Static references request their destination; computed references
request the complete original domain once. Draining before sealing the lowering
retains original function domains and avoids graph-recursive lowering frames.
Indirect references retain the existing exhaustive-deactivation safety guard.
No empty definitions, missing-reference fallback or second validator was added.

static-root-demand-native.log: catalog_references 10, property_references 7,
shape_catalog 6 and values_default_compliance 18 controls pass. Static-root
measurements are exactly equal at 1/8/32: 20 allocations, 3450 requested, zero
retained and 1954 peak bytes. This adds one fixed cold allocation at one definition
but removes unused-domain growth. Absolute caps were added from those measurements
and compiled by the strict check; their latest runtime recheck is still due.
Original catalog admission and 6/544 export/clone ceilings remain intact. Shared
DAG parsing at 1/8/32 is now 827/2432/7727 allocations, 93715/313998/1117822
requested, 13086/47188/168719 retained and 15307/53046/191392 peak bytes.

The native demand command exited 101 only for the nine stale-stage census
assertions (19 census passes). Its 15 carried-analysis source homes include the
queue, request, draining and finish wiring. Actual derived/shipped stage is now
a15e5551dc382415336349dff8cbc68a87c30c6e8e42592092ea950991d84fbf.
static-root-demand-strict.log passed Shapes/validate/Python all-target Clippy
with warnings denied. Every Cargo call uses the Stage wrapper explicitly;
CARGO_REAL selects its pinned inner compiler rather than bypassing the wrapper.

The supported current-stage generator passed restored-report parity and wrote
5608 bytes (static-root-demand-product-generate.log). GOLDEN_LEN is 5608; the
frozen format-v1 artifact has no diff. The larger current fixture versus 6c's
4792 carries the complete definition catalog and property edge records. Historical
745-stage seven determinism/28 census passes are not current-stage qualification.

After adapting old inline inspection/mutation fixtures to original definitions,
all five catalog_product and 24 complete_reports controls pass, including nested
source refusals, changed-source identity and independent same-IRI domains
(flat-catalog-report-product-native.log). Those precede demand lowering and need
changed-input assessment. No acceptance assertion was removed.

Focused native json_schema:: execution initially exposed six real negation
projection regressions (85 passes). Its exact finite negand proof now resolves
original references/property views, retains original domains, refuses dynamic
deactivation proofs and propagates native model/stack failures through the first
error boundary. All 91 schema controls subsequently pass
(flat-catalog-json-schema-native.log). A positive/negative missing-definition
native refusal test was extended and awaits its current runtime recheck.

The ownership adversary failed in native execution: two original models sharing
an index and function IRI both conformed, even though one original function body
returns false. index_position deduplicated by index cell alone. It now keys by
both the index and original owning model, with the same native foreign-index
check on cache hits. All 32 product AST codec controls then passed, including
that before/after-roundtrip adversary (shared-index-domain-native.log).

Further source review found an Owned expression can change the original model
while an enclosing current_catalog still names its caller. Static references now
reuse that fast catalog only when its owning model matches; otherwise they resolve
through their actual original owner. A second native before/after-roundtrip
control gives the same shape IRI different node kinds in the two domains.

The native codec execution passed all 33 controls including both adversaries
(original-domain-codec-native.log). These owner fixes change carried class-analysis source, so a15 and its
fixture are historical until the next actual census derivation/regeneration;
do not reuse them as current qualification. Then qualify planner/linker and
current product/census/absolute allocation caps. Normal
hooks/commit/push remain undone. The full signed SCC/mode/fixed-point contract,
host/provenance plumbing and all eighteen paper cases remain required; two
spec_linker lint controls still refuse the bundled recursive meta-schema. No
complete issue, full gate, PR or merge is claimed.

## Flat parser and property edges: earlier qualification history

Production parsing now queues native identities, parses each body once, moves
definitions into the complete original index, and exports identity handles.
Node, logical/list, member, reifier, someValue, targetWhere, expression and rule
references use that catalog. PropertyReference preserves an actual sh:property
edge and its own overrides; it reuses the existing property evaluator, preserving
child focus/path stamping and constraint-annotation precedence. A shared borrowed
PropertyView serves native report metadata and projections. Rule conditions no
longer trigger a second shape parse with rules suppressed. Standalone property
wrappers no longer clone/evaluate their deactivation expressions twice.

The map admission recipe is shared by authored models, parser linking and product
decoding. It moves original bodies and diagnoses repeated installation, repeated
identities, owning definitions and wrong property kinds with native refusals.
Parser in-flight identities no longer allocate rendered String keys.

Earlier property-reference native execution passed three parity controls. The
real flat parser then passed the same three, plus actual JSON Schema object/member
reference evaluation and shared-DAG admission at 1/8/32 levels. Actual parse
measurements were 829/2434/7730 allocations; 93851/314174/1118382 requested;
13086/47188/168719 retained; 15443/53104/191688 peak bytes. Definitions stay flat,
and small DAGs preserve conformance for foreign IRIs/literals after factory drop.
Linear hard ceilings for all four resources and original missing-definition
projection refusal were added; all six controls passed. After metadata and target
driver changes, the six controls passed again, including the stronger independent
inline-vs-reference nested path-role and serialized report parity check.

Focused consumer execution: catalog_references 10/10, evaluated_targets 28/28,
function_body_relation 26/26, property_references 6/6, shape_catalog 5/5 passed.
The exact original catalog admission/export allocation ceilings remain intact.
spec_linker ran 39 controls: 37 passed; two lint controls refuse because the
bundled recursive SHACL meta-schema still lacks the required explicit mode/SCC
implementation. Those failures remain open under the full issue, not waived.
The first run exposed three actual targetWhere domain omissions; both the
prepared-target and generic target drivers now pass the original model, and the
candidate narrowing walk borrows referenced definitions and effective edge
metadata. All 28 target controls then passed. See flat-catalog-consumer-native.log.

Metadata follows actual identity dependencies, without scanning every retained
definition; original same-IRI nested domains and unused declaration isolation
have native runtime controls. Missing definitions, function declarations and
function bodies return the original typed refusal, with physical stack margins
guarded at inspection entry. Public Rust APIs and the Python error boundary are
fallible. Source-occurrence admission installs original custom bodies too.
Shapes/validate/Python all-target compilation passed before the target-domain
addition. No cross-host runtime claim is made. Product AST codec execution
passed all 31 controls (flat-catalog-codec-native.log).

All-target Shapes/validate compilation passed before the final projection helper
name/root-transparency adjustment. Native schema evaluation passed before that
adjustment. Catalog helper keys do not enter the semantic class-definition set,
avoid reserved GraphQL names and reserve ontology fragment keys. An exported
root is resolved transparently before object-schema projection. Missing references
must return the first original typed failure, never a partial compiled artifact.

Still required on this follow-on: final strict checks; graph-aware original
scope review; broader affected expression/schema controls; updated model census,
product stage/fixture and generated metadata after semantics settle. No follow-on
commit, full parser/solver qualification, PR or issue completion is claimed.

Subsequent model/value execution passed seven property/alias controls; the
computed-value target exposed seven real old-layout rule-driver failures (11
passes), and the census exposed the expected nine stale-stage assertions (19
passes). These were diagnosed and fixed, not rebased away. Rules now borrow the
original shape body and prepared property edges, including both shape/edge
deactivation. Discovery reads the original per-focus deactivation law too.

Native rules recheck passed values_default_compliance 18, rules_engine 36,
rules_conformance 3, rules_serialization_roundtrip 1, property_references 7 and
box_role_alloc 5 controls. The only failed target was the deliberately stale
stage census. The carried class-analysis census now includes lower_shapes_in_model
and lower_reference, the two extracted production homes. Its actual derived
stage is 745b0b8d522d49c423f0bd0195b2aee706a87ab646d39f48a0a90e2603e23d4b;
the shipped constant has been updated from that evidence, pending final rerun.
See flat-catalog-rules-native.log; its overall exit is 101, not a full pass.

Strict Shapes/validate/Python all-target Clippy passed with warnings denied
(flat-catalog-strict.log), before the final schema cache key adjustment. The
cache now uses runtime original-body identities on hits and retains the original
term once per emitted definition, eliminating repeated owned term keys without
losing the collision's native subject. Reifier contexts are borrowed; their
report-only role material is not built for conformance-only sinks. No lint allows
were added. Recheck the schema change and current strict result before committing.

That snapshot's supported fixture generator subsequently passed actual
restored-report parity; subsequent current-stage work is recorded above.

## Model-owner integration: 6c767c864

The preserved donor is integrated into the surviving checkout, with all seventeen
conflicts evaluated. All donors and the exact original stash remain. One
ModelOwner replaces separate owner/catalog types, fields and codecs. Logical
calls use IRIs, computed references are weak, and original declaration/domain
identities key native bodies and reporting guards. Function lookup searches the
moved sorted boxed slice. Product collection transfers its identity table
directly to the writer, avoiding another map allocation pass.

| Behavior | Actual qualification and limits |
|---|---|
| Direct evaluated arguments | borrowed-call-result-final.log: two actual controls pass. At 1/8/32/512 arguments, constant five allocations, at most 214 requested/peak and 28 retained bytes; consuming the last argument passes. A 1 KiB/1 MiB returned literal uses six allocations and payload + 222 requested bytes, with one output ownership copy. Hard ceilings retained. |
| Optional holes / processor failures | borrowed-call-values-semantics.log and model-owner-processor-native.log pass actual prebinding, typed refusal and recovery. |
| Function admission | At 1/8/32 declarations: constant two allocations and 96 requested/retained/peak bytes. |
| Authored catalog | Final integration passes five allocation/report/refusal cases. At 1/8/32 definitions: 4/11/35 allocations; 1686/6416/25350 requested; 1374/3920/15366 retained; 1590/6320/25254 peak bytes. The full domain adds a fixed 16-byte declaration-slice header versus bd511379f; bounds account for it separately, preserving removed growth. Root export+clone remains six allocations/544 bytes. |
| Original scopes and release | Final integration passes eight reference/domain/report/release cases, including nested same-IRI function/relation inventories. |
| Native index use | model-owner-link-final.log: thirteen controls pass. Populate for actual weak computed references or an explicit native shape reader. Unused indexes initialize empty without copying root bodies, fixing the new owner's strong-reference regression. |
| Product domains | model-owner-product-final.log passes all five product-domain controls after the final writer-table transfer, including distinct same-IRI function bodies and original definition domains. |
| Codec ownership | model-owner-codec-native.log passes all 31 controls after the final identity-table transfer. The hostile owning-definition fixture declares its actual weak self-reference before injecting a strong cycle; the unused-index optimization no longer changes its premise. No assertion or admission check was removed. |
| Product identity | All 28 model census cases pass. Derived stage identity: 160673e2cb397c2fbb4df53a41e7f5622b4efea5aa2244d97f628585dab57247. |
| Frozen product | Supported generator passes restored-report parity before writing 4792 bytes (formerly 4744). Final writer code passes all seven ordinary determinism cases, frozen identities and historical rebuild controls. The generator was run explicitly and is intentionally ignored in the ordinary suite. No frozen cross-engine GTS vector changed. |
| Strict checks | Final Shapes/validate all-target Clippy passes with warnings denied (model-owner-strict.log). Owned whitespace and normal commit hooks pass. Commit 6c767c864 and its normal push both passed; the checkout is clean (model-owner-commit.log, model-owner-push.log). |

The complete flat RDF parser remains unfinished. A preparation test initially
added a flat reference to a parsed definition with no declared dependency; that
currently returns typed UnknownReference. The export control now starts with an
actual computed dependency and replaces that edge with its flat identity form.
It proves original-domain retention, not arbitrary otherwise-unreferenced
parser definitions. Complete once-parsed admission, all original function scopes,
SCC/LFP/GFP, all eighteen paper cases, actual hosts/options/provenance/schema
support and final independent review/delivery remain required. No partial PR or
issue completion is claimed.

## Previously committed preparation

3be134b34 integrates current main and single-owner call arguments. Normal hooks
and push passed. Initial call allocations at 1/8/32 fell from 17/31/81 to
15/22/48; newer borrowed-value evidence above supersedes those figures. Native
processor classification, stack refusal/recovery and strict Shapes/ShEx/validate
all-target Clippy passed. See main-sync-* logs, scoped to that input.

bd511379f prepares authored identity catalogs. Normal hooks and push passed.
Admission at 1/8/32 fell from 5/21/71 to 4/11/35 allocations and
1672/11164/49064 to 1670/6400/25334 requested bytes. Root export+clone fell from
8/36/132 allocations and 606/4568/18152 bytes to constant six allocations/544
bytes. Four native catalog controls and strict Shapes/validate checks passed.
The borrowed sink fixes recursive generic instantiation without wrapper/buffer
allocation; actual serialized reports preserve caller path-role vocabulary.
Original UnknownReference operands survive disposal and error cloning costs
zero allocations. See catalog-native-final.log, catalog-strict-final.log,
catalog-commit.log and catalog-push.log. These facts are scoped to that commit.

Source reconciliation and primary paper evidence remain in reconciliation.md
and raw/. Table 1 has eighteen cases; no solver acceptance is inferred from
reading it. No discretionary broad runtime or WASM run was used.
