# Issue #400: Opt-in remembered empty named graphs for the mutable dataset and SPARQL Update

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement

## Body

## Problem
The mutable dataset and SPARQL Update follow a documented "a named graph exists iff it holds a quad" store model. SPARQL 1.1 Update permits that model, but it cannot represent empty named graphs: `CREATE GRAPH` does nothing, `CLEAR` and `DROP` behave the same, and ADD/COPY/MOVE cannot create or keep empty destinations. RDF 1.2 dataset fidelity needs graph existence that does not depend on rows.

## Proposed solution
Add an **opt-in, additive** mode (a new option or constructor; the default stays the same in 3.x) that remembers empty named graphs:
- CREATE registers an empty named graph; CREATE of an existing graph fails unless SILENT.
- CLEAR removes rows and keeps the declaration; DROP removes the declaration.
- ADD/COPY/MOVE follow the spec for absent, empty, default, SILENT and self transfers. A missing COPY/MOVE source must not destroy the destination.
- Capability flags and `GRAPH ?g` enumeration report declared empty graphs.
- SILENT never swallows a governor trip or cancellation. Publication polls the governor after freeze and before swapping in the new dataset.
- Docs state that this mode is expected to become the default in v4.0.

Build on the existing homes: `RdfDatasetBuilder::declare_named_graph`, the frozen graph registry in `ir/dataset.rs`, and a private suppression set in the mutable delta. Do not add new collection types or physical-allocation accounting. If new charge points are needed, they get a new governor profile version and do not reuse an existing number.

## Acceptance
- One Rust test per operation × source/destination state, in both modes; default-mode behaviour is unchanged.
- For every new refusal, a neighbouring valid case still succeeds.
- `cargo semver-checks` against rust-v3.0.1 passes; `make check` and `make wasm` pass.


## Comments (9)

### paudley — 2026-10-04T05:53:50Z

Scope addition (maintainer decision): the Python `Store` / `purrdf.parse` surface is a quad-list model with no graph-declaration API, so `Store.load(TriG)` followed by `Store.dump(TRIG|JSON_LD)` drops declared empty named graphs such as `<g> {}`. Add graph-declaration state to the Python Store (e.g. `add_graph` / `named_graphs`) in this issue, so the Rust and Python graph models land together. Acceptance: a Rust-owned test drives the installed Python extension through load→dump for TriG, JSON-LD and TriX, with and without the opt-in mode.

### paudley — 2026-10-04T13:50:12Z

Scope update (maintainer decision): the part of this issue's Python item that loads and dumps declared empty named graphs is delivered by PR #410. `Store.load` and `MutableDataset.load` keep declared empty graphs (through the additive `MutableDataset::declare_named_graph` / `declared_named_graphs`), and `dump` / `dump_with_loss` write them wherever the target format can. The Python item that remains here is the opt-in remembered-empty-graph API under the opt-in mode: registering a graph with CREATE semantics, keeping a slot through CLEAR, and the `add_graph` / `named_graphs` surface under that mode, tested through the installed extension.

### paudley — 2026-10-05T13:23:51Z

Merged in #410: declared empty named graphs now survive JSON-LD, YAML-LD, TriX and PACK, and are recorded as `empty-named-graph-dropped` losses on targets that cannot express them. This issue still owns the opt-in remembered-empty-graphs mode for the mutable dataset and SPARQL Update.

### paudley — 2026-10-05T18:14:24Z

# Complete plan: remembered empty named graphs

Issue: https://github.com/Blackcat-Informatics/purrdf/issues/400

## Issue summary and current authority

Deliver an additive, explicitly selected graph-existence mode for the mutable
dataset, native SPARQL Update and installed Python Store/MutableDataset surfaces.
The existing 3.x mode and zero-argument entry points retain their graph-operation
answers. In the new mode, graph declarations survive an empty graph and CLEAR;
DROP removes the declaration. All request publication remains atomic.

Read the complete live issue and its three maintainer comments. The comments
5977090983, 5980699600 and 5995362152 require the installed Python graph API, but
explicitly assign load/dump fidelity to the already merged serializer work.
That work merged as b8ef31ffc and is contained in the captured main base; it
will be consumed unchanged here.
The original load/dump formats remain integration witnesses, not codec edits.
Issue 401 remains a separate requirement for document blank remapping and
typed record transfer; this branch supplies its graph-mode prerequisite.

Root authorized isolated setup and this plan/source investigation only. No
implementation, compiler, test, commit hook or benchmark starts before BOTH root
plan approval and the explicit release of the coordinated timing window.
No GitHub claim/plan message has been posted during this local review.

## Baseline defaults

Read root `.baseline`: adversarial source/acceptance review; complete delivery;
no stubs, omitted acceptance or unresolved emergency entries; no development or
issue identifiers in repository product documentation. Process receipts belong
on GitHub and in task evidence outside the source tree.

## Standing constraints and ownership

Read root `.goals` and current AGENTS.md. The work is Rust first and RDF 1.2
complete; the existing first-party homes, fixed-key maps and collection types are
used. No dependency, semantic Cargo feature, fabricated vocabulary, runtime RNG,
physical-allocation accounting platform or new general graph framework.
Python/JavaScript only exercise their actual installed surfaces; assertions and
the new matrix are native Rust. No generic WASM semantic replay is added.
Preserve old wire/vector bytes, default constructors and public closed types.
Normal mandatory hooks are never bypassed; signatures and source identities bind
all claims. The emergency ledger currently has exactly its notice and marker.
No CONSTITUTION.md or nested AGENTS.md was found in the assigned source tree.

Fresh immediately-before-setup ownership checks covered every local branch and
worktree, live remote branches, all open PR heads/bodies, and the complete live
issue comments/assignees. No 400 ownership existed. Preserve protected PRs
428/436/437/440/441, other-owner 442/444, root 406, numeric 403, sibling 402 and the
existing recovery/release lanes. A branch without a PR is ownership. Refresh
those checks again before any new issue intake. No new intake is part of this plan.
Toolchain investigation/configuration/selection/adapters/install/removal and
host/disk/process/cgroup/cache inspection or cleanup are forbidden. Only normal
repository commands are used. Cargo commands use ordinary `--jobs 4` arguments.

## Completeness contract

| Live requirement | Delivery and proof |
|---|---|
| Opt-in additive mode; existing 3.x default unchanged | T1: new mode type and sibling constructor; T2: request option; old registration/body and full legacy graph-existence suite retained; independent default-mode matrix |
| CREATE registers absent graph; existing empty/populated graphs fail except SILENT | T1/T2: effective membership and duplicate diagnostic; named native cases for every state and SILENT neighbor |
| CLEAR retains declaration; DROP removes it | T1/T2: one registry/withdrawal home; named, DEFAULT, NAMED, ALL cases, base/delta origins, blank graph names |
| ADD/COPY/MOVE absent/empty/default/SILENT/self behavior | T2: explicit state table; separately registered Rust trials for each valid operation/state/mode/SILENT combination and self case |
| Missing COPY/MOVE cannot destroy destination | T2: source existence before destination mutation; whole-state image and Arc identity assertions on refused requests; SILENT no-op and valid empty-source neighbors |
| Capability flags and GRAPH ?g include declared empty graphs | T1/T2: complete registry feeds frozen named_graphs bit; effective snapshot flag; frozen, same-request variable and constant GRAPH probes agree |
| SILENT never absorbs governor trip/cancellation | T3: semantic refusals alone become no-ops; typed tripped state escapes; deterministic fuel/cancel/deadline neighbors preserve original handle |
| Poll after freeze, before publication | T3: freeze to a local value, final stop/trip check, then sole assignment; failing-first countdown and admitted neighbor |
| Existing builder/frozen registry/private suppression set homes | T1: existing declared_graphs/withdrawn_graphs/graph_rows; no second registry or collection class |
| No physical accounting/new collection types; new charge points require new profile | T3: no new charge point or schedule/closed-enum change; existing stop checkpoint change reviewed and versioned under published law below |
| Document expected v4.0 default, preserve opt-in 3.x | T4: Rust/Python API and store-model documentation; no current default flip |
| Python remaining add_graph/named_graphs and remembered CREATE/CLEAR API | T4: shared PyQuadStore implementation, keyword constructor selection, mode retained across every rebranch; installed-extension Rust driver |
| Maintainer load/dump scope correction | T4: consume merged loader/serializers, prove TriG/JSON-LD/TriX crossings with both modes without another codec implementation |
| One Rust test per operation × state in both modes; refusal neighbor | T1-T4: uniquely registered native matrix and fault cases; inventory exact case names/counts from real harness output |
| semver against rust-v3.0.1 | T5: core/RDF/umbrella/eval exposed surfaces, original/current JSON source guards if artifact comparison is used |
| make check and make wasm acceptance | T5: all existing native prerequisites/recipe payloads plus the exact release-WASM Cargo payload; literal wrappers NOT RUN because they inspect Rustup; source-bound native/Miri/portability receipts and honest separately waived WASM CI |
| Complete staged delivery | T0-T7: root plan review, signed task commits/hooks/push/issue receipts, non-draft PR, complete CodeRabbit response and final-head CI, current-main sync, root-granted ghprsq and audit/cleanup |

## Enhancements considered and declined

The local plan/compliance/enhancement audit is performed without delegating;
root is the independent reviewer under the current single-level authority.

* Transformation, adopted (M/high): model a graph as a mandatory default slot
  plus an independently present named slot, rather than infer existence from
  rows. This is precisely the existing frozen registry's job. It subsumes CREATE,
  CLEAR, DROP, empty transfer and query enumeration without a parallel store.
* Leverage, adopted (S/high): carry the one core mode through request options and
  Python rebranching, so checkpoint/compact/UPDATE cannot silently reset it.
* Utility, adopted (S/high): check variable and constant GRAPH forms during the
  same request as well as after freeze; expose typed graph names in Python.
* Robustness, adopted (S/high): reject a missing transfer source before touching
  the destination; preserve the mandatory default slot and explicit COPY/MOVE
  self exceptions, while separately defining ADD-self missing-input admission.
* Robustness, adopted (M/high): independently enumerate the complete operation
  matrix, metadata-only cancellation, base/delta declarations, blank graph names,
  last-row removal, snapshot isolation, and post-freeze publication.
* Declined: persistent mode bits or new wire sections. Policy is chosen by the
  mutable/request caller; frozen graph presence is already complete data.
* Declined: second graph registry, new map class, physical accounting or another
  Python state model. Existing native state owns the complete behavior.
* Declined: broad CLI/C/WASM option expansion or a v4.0 default flip. The live
  acceptance explicitly requires additive Rust/Python selection in 3.x; the other
  entry points keep their defaults and shipping builds.
* Declined: LOAD document identity/typed transfer redesign. It belongs to the
  separate issue 401, whose complete requirements are not claimed by this PR.
* Declined: reimplementation of merged load/dump or PACK behavior. Consume and
  test the existing carriers and preserve their byte laws.

## Design: one state home, additive selection

Add non-exhaustive `GraphExistenceMode` in the existing mutable IR home, with
`Implicit` (Default) and `RememberEmpty`. `MutableDataset::new(base)` remains
implicit. Add `new_with_graph_existence(base, mode)` and a read-only mode getter;
there is no mid-branch mode switch. Reexport the type beside MutableDataset in
core/IR/RDF; the umbrella's existing reexport carries it. Do not modify the
protected model trait implementation or any serializer/PACK implementation.

Add effective named-graph membership using the existing base dictionary/registry,
live row counts, declarations and withdrawn set. In RememberEmpty, first insertion
into a named graph creates its declaration through the existing home; last-row
removal retains it. Explicit DROP withdraws it after row removal. CLEAR removes
rows without withdrawing. Implicit mode follows the current withdrawal law,
including the declared-input-empty carve-out pinned by existing tests.

Put mode-aware CREATE admission in one additive native
`MutableDataset::create_named_graph(graph)` method alongside the existing
idempotent `declare_named_graph`. It validates graph ingress and applies effective
duplicate/mode rules, returning a diagnostic on a remembered duplicate. Both
SPARQL CREATE and PyQuadStore.add_graph call it; they only apply their own
SILENT/exception presentation. Existing declare_named_graph stays idempotent and
unchanged in every mode, so the merged loader remains compatible. Neither host
reimplements the graph-presence or duplicate law.

Mode is policy, not serialized data. Freeze/snapshot carry only graph presence
through the existing builder. Retained old snapshots never change with their
mutable branch. A native sibling branch explicitly receives the chosen mode.

Current builder capability calculation examines only graph-bearing row tables,
so a declaration-only frozen graph reports named_graphs=false. Repair that bit
from the complete existing registry in the existing builder capability home.
Derive the effective snapshot's named_graphs bit from its effective registry;
do not blindly retain a base bit after all graphs are dropped. No other capability
bit or persisted representation is redesigned. Check ordinary, reifier-only,
annotation-only, declaration-only and removed-all neighbors. Any observable
existing wire mismatch is investigated and reported before accepting a golden;
existing wire goldens are never silently rewritten to accommodate this feature.

Add `QueryOptions::with_graph_existence(mode)` and the corresponding additive
field in its already non-exhaustive struct. EMPTY/new remain Implicit. Both
ordinary and governed UPDATE construct their mutable branch with that exact
selection. Query execution simply reads the complete frozen/view registry and
does not need a mode. Existing engine trait signatures, result shapes and global
defaults are unchanged.

## Exact operation/state semantics

Normative references: W3C SPARQL 1.1 Update sections 3.1.4, 3.1.5 and 3.2.1-3.2.5,
https://www.w3.org/TR/sparql11-update/ . Preserve the specification's mandatory
default graph. The chosen missing-source refusal is permitted by its MAY rule
and required by the issue's destination-protection acceptance.

For RememberEmpty:

| Operation | Effective source/target state | Result |
|---|---|---|
| CREATE named | absent | declare empty |
| CREATE named | declared empty or populated | graph-exists diagnostic; SILENT leaves whole state unchanged |
| CLEAR named | absent | graph-missing diagnostic; SILENT unchanged |
| CLEAR named | empty/populated | retain slot, remove all rows |
| DROP named | absent | graph-missing diagnostic; SILENT unchanged |
| DROP named | empty/populated | remove rows and slot |
| CLEAR DEFAULT / DROP DEFAULT | empty/populated | empty mandatory default; preserve every named graph |
| CLEAR NAMED / ALL | no named slots, empty-only, mixed | empty applicable rows, retain named slots; ALL also empties default |
| DROP NAMED / ALL | same aggregate states | remove named slots/rows; ALL empties but retains default |
| ADD distinct targets | missing named source | refuse before mutation; SILENT unchanged |
| ADD distinct targets | empty/populated source | retain source; create named destination even for empty input; union destination rows |
| COPY distinct targets | missing named source | refuse before destination clear; SILENT unchanged |
| COPY distinct targets | empty/populated source | retain source; replace destination rows and keep/create its slot |
| MOVE distinct targets | missing named source | refuse before destination clear; SILENT unchanged |
| MOVE distinct targets | empty/populated source | replace destination, keep/create its slot; drop named source or empty mandatory default |
| COPY/MOVE self | absent/empty/populated named or default | explicit spec exception: no-op before missing-source check; state and mutation charges unchanged |
| ADD self | existing empty/populated named or default | source admitted; union with itself leaves its slot/rows unchanged, with no row mutation |
| ADD self | missing named source/destination | selected missing-input refusal before any destination creation; SILENT succeeds with slot still absent |
| INSERT DATA / INSERT WHERE / nonempty LOAD | newly populated named graph | establish slot and retain it after later last-row removal |
| DELETE DATA / DELETE WHERE / DELETE-INSERT | last row removed | keep slot; unrelated slots and retained snapshots unchanged |
| Successful empty LOAD into named | absent/empty/populated destination | establish/retain destination and existing data |
| LOAD operational error | any destination | existing error/SILENT/governor law unchanged; no new declaration from a failed load |

Implicit mode is an explicit separate expected table preserving current behavior:
CREATE succeeds without registering; CLEAR and DROP both withdraw; empty ADD
creates nothing; empty COPY/MOVE remove the cleared destination; missing-source
transfers keep their current answers; every existing legacy case remains. All
implicit self operations remain their historical unconditional no-op. This
compatibility table will not be inferred from the new production helper.
Graph existence counts every RDF row role; graph-state tests include side-only
and blank-named graphs without claiming the separate typed-transfer repair.

ADD-self rationale is explicit rather than borrowed from COPY/MOVE. The live
issue/comments require spec-compatible self behavior and unchanged default mode;
none imposes an unconditional ADD-self exception. W3C Update section 3.2.5 gives
ADD its INSERT-WHERE equivalence, destination creation, and the permitted
missing-input failure policy; sections 3.2.3/3.2.4 separately give COPY/MOVE an
explicit self exception. RememberEmpty uses the same selected missing-source
refusal for ADD whether the destination name differs or matches. Refusal happens
before destination creation, and SILENT suppresses only that semantic failure,
leaving the missing graph absent. A present empty named self ADD and default-empty
self ADD succeed without removing/creating a slot; populated self ADD preserves
all rows. Implicit retains the current source guard at update.rs1057-1060. The
native matrix registers each of those distinct outcomes and their SILENT valid
neighbors; no nonexistent-graph no-op is justified by an invented ADD exception.

## T0: isolated setup (complete)

Worktree `/home/paudley/Active/purrdf/.worktrees/400-remembered-empty-graphs`.
Branch `paudley/400-remembered-empty-graphs`.
Fetched base `33bc4f2f12e9c5dd874a88c3904d40217c763ab4` (origin/main).
The normal worktree creation exited 0; assigned worktree is clean. Root main and
sibling worktrees are untouched. Only planning evidence is written outside source.
No compiler/test/hook/benchmark was launched. Preserve this hold for root review.

## T1: native mutable mode, registry and capability proof

After clearance, record real pre-change witnesses for CREATE/last-row empty-slot
loss and the declaration-only capability bit, using existing public paths. Build
the mode/constructor/API and the complete state-home changes together with their
tests; never commit a knob whose selected behavior is unfinished.

Native tests independently assert declared graph sets, role row counts, capability
bits, exact default graph existence, base/delta origins, scoped blank graph names,
remove-last/reinsert, freeze/snapshot and retained-snapshot isolation. Preserve
all legacy mutable/named-graph/pack/canonical fixtures and old registrations.
Use existing collection/term APIs and checked existing resident bounds.

Measure immutable pre-change existing mutable benchmark cases before making
performance claims, only in a root-coordinated window. Extend that existing bench
for declared-empty/last-row/clear/drop workloads in both modes. No fixture, sample,
threshold or build-flag changes. Avoid an extra allocation in the implicit path.

Focused commands include normal locked core tests for mutable and named graph
targets, existing mutable_declared_graph_drain/pack_graph_declarations/canonical
targets and warning-denied affected Clippy. Capture source/command/exits/counts.
Commit T1 with normal mandatory signed hooks, push, read back and post an issue
receipt only after this whole task passes. Do not install or bypass hooks.

## T2: native UPDATE selection and complete matrix

Wire both engine doors to QueryOptions and distinguish CREATE/CLEAR/DROP, honoring
SILENT only for semantic failures. Shared target/membership homes perform source
validation before COPY/MOVE destination mutation. Keep COPY/MOVE self checks
first and preserve the Implicit ADD self guard; in RememberEmpty, admit ADD source
existence before a present-source self optimization.
Graph metadata for empty successful LOAD is the only LOAD change here.

Keep existing update_graph_existence.rs assertions/body and register a new Rust
matrix target with the existing testkit harness. Each operation/state/mode/SILENT
combination is a distinct uniquely named Trial, not one opaque test looping every
case. Expected state is an independent slot-and-row model, not production calls.
TRANSFER source/destination states are named absent/empty/populated and default
empty/populated. The 5-by-5 logical grid contains 21 realizable distinct-target
state pairs (nine named-to-named, six named-to-default, six default-to-named).
The four default-to-default grid entries cannot describe distinct targets; they
are covered by the two consistent default self states. All 21 distinct pairs,
both SILENT forms, both modes, and the five realizable named/default self states
for each ADD/COPY/MOVE are covered. The transfer inventory is exactly 312
registered cases: 21 distinct pairs x 3 operations x 2 modes x 2 SILENT forms
plus 5 self states x 3 operations x 2 modes x 2 SILENT forms. The native
inventory asserts both the independent per-family counts (252 and 60) and all
actual unique generated names; actual harness listing/run counts must agree.
CREATE covers the three named states. CLEAR/DROP cover each named/default state and empty-only/mixed/none
aggregate NAMED/ALL cases. Include every DATA/WHERE last-row operation and LOAD
success/error neighbor. Derive and publish the actual exact inventory/counts.

Each case compares full content and declarations after freeze, variable and
constant GRAPH queries, and a trailing WHERE operation in the same request.
Failed requests assert original Arc pointer and complete-state image unchanged.
Preserve default historical expectations and all parser/OPTIONAL owner boundaries.
Register diagnostics through the existing diagnostic documentation/metadata homes.

Run the new matrix, all existing update_graph_existence/governed_update cases and
the native conformance update/governor targets, then affected all-target Clippy.
Normal signed task commit/hooks, push/readback and source-bound issue receipt.

## T3: governor refusal and exact publication law

Freeze into a local Arc, then poll/check the same GovernorState, and assign the
caller dataset only if clear. All earlier non-success paths discard the branch.
SILENT never catches UpdateAbort::Tripped or reclassifies cancellation/deadline.
Check stop in new declaration-only loops, including bulk NAMED/ALL work. Keep
row mutation charges unchanged and report only the work actually performed.

No new charge point is proposed. ChargePoint is currently a public closed enum;
CHARGE_SCHEDULE and NodeCharges expose fixed lengths. Growing or repurposing them
would violate additive compatibility. Graph declarations are not quad mutations.
No new fuel meaning, schedule entry, dimension, physical accounting or governor
option is added. Metadata response to stop is tested separately from row fuel.

Root-approved version choice: advance the existing profile version
once (currently 11, thus 12 if no intervening accepted profile change), keeping
the charge schedule byte-identical. Exact published law is
docs/SPARQL-GOVERNOR-PROFILE.md section 12, lines 654-665, and
governor/mod.rs1413-1420: a change that can move the budget trip/cut point changes
the profile. poll_stop1032 calls poll_stop_after, whose existing law reports one
checkpoint when no pending fuel exists. The added post-freeze checkpoint can
observe a stop the old publication path missed; added metadata-loop polls alter
that same observable stop schedule. StopSignal::poll_after_work at lines 133-151
requires each no-fuel checkpoint to report its own one unit, and the published
v9 history explicitly records moved poll placement with byte-identical fuel
schedule. That is the concrete basis for this conservative version proposal:
observable stop/publication cut points move, while numeric fuel costs do not.
No global query checkpoint or shared default pricing contract is broadened merely
to price metadata. The final publication check applies to both modes because it
is separately required by the issue. This is a publication/cut-point repair, not
a new ChargePoint. Root approved this precise choice; implementation still waits
for the revision readback and explicit measurement-window release.
The caller types, precedence, inclusive fuel boundaries and existing per-row
costs remain unchanged; profile-pinning consumers see an honest new identity.
Do not add a legacy-profile execution selector or change query evaluation costs.

Native deterministic tests use the existing PollCountdown/latching signals:
trip at each new metadata/publication boundary, immediately adjacent quiet-count
success, CREATE SILENT duplicate plus cancellation/deadline, missing COPY/MOVE
SILENT plus fuel/stop, zero/exact/one-short row fuel and multi-op rollback.
An existing public UPDATE countdown fails first on the old missing final poll:
it stays clear through the old final pre-freeze poll and fires at the post-freeze
checkpoint; compare Applied vs BudgetExhausted, complete state and Arc identity.
Structural unit/source proof confirms actual freeze precedes that exact poll.
No wall-clock race, host inspection or production test hook is needed.

If the profile version changes, update its existing normative documentation,
identity pin and authored governor-corpus metadata through the existing tools.
Keep schedule tuples/indices/types and old semantic vectors intact; add the
actual new publication/refusal vectors and prove full XPASS/count/freeze behavior.
Run complete native governor/update tests, profile digest/charge tests and affected
Clippy. Commit/push normally and post the actual task receipt.

## T4: shared Python API and Rust-owned installed-extension proof

Add keyword-only `remember_empty_graphs=False` to Store and MutableDataset
constructors, selecting the exact native mode. Implement `add_graph(graph,
silent=False)` and `named_graphs()` once on PyQuadStore using native membership,
declaration and graph term conversion. add_graph follows CREATE: absent is
registered in remembered mode; duplicate raises ValueError unless silent;
implicit mode keeps CREATE's no-op behavior. Native graph API accepts the existing
IRI/blank graph name space; SPARQL syntax remains IRI-only. Invalid graph kinds
and relative IRIs are refused through existing ingress homes with valid neighbors.

Capture mode before every rebranch and restore it with the sibling constructor:
ordinary/governed UPDATE, Store.checkpoint, MutableDataset.compact and any shared
adoption helper. Pass the same mode into both engine UPDATE doors. Trip paths
preserve both the original branch and selected mode. Reuse existing loaded graph
declarations and serializer/dump_with_loss code unchanged. No parallel Python
graph list or new semantics in Python source.

Rust owns installed-extension expectations in a host integration target under
the CLI test surface, not under extension src (its test=false PyO3 library cannot
link a native test executable). A short observation-only python3 -c bridge calls
the installed package and returns native graph names, dump bytes, exceptions and
mode observations. Rust constructs fixtures, parses the observation and asserts
all expectations. No new Python test file or general bridge framework.
Register `python_empty_graphs` as a native `harness = false` CLI integration
target using the existing testkit runner, with one ignored trial named
`installed_empty_graph_modes`. It is ignored only in ordinary workspace runs.
The existing `make pytest` recipe retains its locked sync and full Python suite,
then runs this exact Rust command from the same binding project:

```
cd bindings/python && PURRDF_TEST_REQUIRE_EXACT=1 uv run --locked cargo test --manifest-path ../../Cargo.toml --jobs 4 --locked -p purrdf-cli --test python_empty_graphs -- --ignored --exact installed_empty_graph_modes
```

The Cargo process and its Rust test executable are children of `uv run` in the
project that installed the extension; the short `python3 -c` observation bridge
inherits that virtual environment's interpreter PATH. It must import the real
installed native package and report its module identity; missing import or a
wrong installed API is a test failure, never a skip. The existing testkit exact
selection guard requires exactly one executed requested trial and refuses a
missing/renamed/ignored selection. Capture the actual one-case run result.

The current pytest CI step directly runs uv rather than `make pytest`; change
only that existing test step to invoke `make pytest` from the repository root.
This gives local and hosted installed proof one command home and preserves the
existing install/full-suite payloads. Do not add a job, Python test framework,
toolchain setup, cache change or general WASM test. Verify the real final workflow
runs the mandatory Rust command and reads its exit, not merely a Makefile path.

Run both class constructors/default compatibility; CREATE/add_graph duplicate
and SILENT neighbor; CLEAR vs DROP; update/update_governed; checkpoint/compact;
TriG/JSON-LD/TriX load-dump crosses in both modes and declared-empty queries.
The already merged load/dump behavior is an unchanged integration dependency.
Update shipping .pyi/API docs, Rust rustdoc and store-model book prose, including
the expected v4.0 default while documenting the actual 3.x default. Regenerate
metadata/i18n from source when needed; no issue/PR identifiers in product docs.
Run actual locked installed-extension test, full make pytest, normal stub/type
checks, affected Clippy/docs and i18n/generated gates. Commit/push/task receipt.

## T5: complete source-bound qualification

Run sequentially under team coordination; never compete with an exclusive
benchmark window. Commands use the repository's unmodified normal toolchain.

1. Full relevant native core/eval suites, complete new matrix and full governor
   corpus; exact counts, zero unexpected failure/XPASS and retained old vectors.
2. Installed Python integration plus full make pytest; warnings/type/selector
   registration checks. No generic semantic WASM execution expansion.
3. `cargo clippy --jobs 4 --locked -p purrdf-core -p purrdf-sparql-eval -p
   purrdf-python --all-targets -- -D warnings` and warning-denied docs on exposed
   packages. Normal `make metadata` and generated/i18n verification.
4. `cargo semver-checks check-release -p <surface> --baseline-rev rust-v3.0.1
   --release-type minor` for core, RDF, umbrella and eval. If the normal supported
   artifact comparator is used, create own current JSON with ordinary Cargo,
   prove original rust-v3.0.1 source/artifact identity, and compare explicitly.
   Never manufacture a compiler/baseline identity or change closed API shapes.
5. The literal `make check` and `make wasm` wrappers are NOT RUN: check ends
   by invoking wasm, and wasm first calls the forbidden `rustup target list`
   inspection. Do not invoke, patch, bypass or configure those wrappers. Execute
   the existing `make node-prerequisite binaryen-prerequisite` payload and every
   native check recipe
   payload directly, preserving order, arguments and assertions; expand the
   existing rdf-core-hygiene payloads normally. The existing toolchain-pin hygiene
   script is pure committed-file text (its line 45 explicitly says no cargo,
   network or rustup), so its normal unchanged payload remains included. Capture
   the final integrated Makefile SHA and an exact indexed command/exit inventory.
   Use ordinary --jobs 4 only on Cargo commands that accept scheduling; no new
   environment override, toolchain selection/configuration or workaround.

   Then execute the exact existing release-WASM Cargo payload below normally,
   preserving --locked/--release/target/--lib and every package. No Rustup check,
   target installation, toolchain probe, alternate flags or silent skip precedes
   it. Capture actual exit/source/head/tree; a missing ordinary target is a real
   recorded blocker, not permission to change it. The source-law receipt freezes
   the current wrapper/native/payload mapping; refresh that mapping from actual
   final source rather than reuse a stale command list.

   Hosted native/workspace/docs/MSRV/Miri/Python/conformance and ordinary shipping
   portability gates must qualify the exact final head. User-waived solely-WASM
   timeout/aggregate failures are recorded as waived, never reported passing.
6. Existing before/after mutable bench cases and new metadata workloads using
   immutable same-fixture normal harness artifacts/samples and coordinated windows.
   Report actual measurements and limits; no global performance claim.
7. Deficiency/deferral/SPDX/hygiene/ratchet/helper/layer/domain/wire-golden scans.
   Every finding is fixed or an explicit root-visible blocker; no incomplete
   feature is committed or presented as qualified.

Exact captured release-WASM Cargo payload (only ordinary scheduling added):

```
cargo build --jobs 4 --locked --release --target wasm32-unknown-unknown --lib \
  -p purrdf-events -p purrdf-lex -p purrdf-iri -p purrdf-xsd -p purrdf-cdt -p purrdf-jsonschema -p purrdf-hash -p purrdf-deflate -p purrdf-stack -p purrdf-ed25519 -p purrdf-gts -p purrdf-core -p purrdf-columnar \
  -p purrdf-datalog \
  -p purrdf-sparql-algebra -p purrdf-sparql-results -p purrdf-sparql-eval -p purrdf-hnsw \
  -p purrdf-rdf -p purrdf-markdown -p purrdf-json -p purrdf-slice -p purrdf-shapes -p purrdf-shex -p purrdf-entail \
  -p purrdf-geo -p purrdf-text -p purrdf-retrieval \
  -p purrdf-validate -p purrdf -p purrdf-wasm \
  -p purrdf-bench
```

Full native/Clippy/doc checks already proved on the exact final hosted tree may
be reused as explicit component receipts when root agrees; do not relabel old
local evidence or repeat an expensive proven suite without a concrete gap.
The live issue's complete check/wasm requirements remain mapped and visible.

## T6: signed delivery, non-draft PR and complete review

Each completed task has a logical signed normal-hook commit, push, readback and
issue receipt. After all requirements/gates pass, create a non-draft project PR
with Closes #400, final behavior and actual validation; post the approved plan
and feature acceptance mapping to issue and PR. No tool/AI branding.
Retrieve all CodeRabbit summaries/inline threads and issue/PR comments. Respond
with concrete source/test evidence; fix real findings in their owning homes,
rerun affected gates, signed commit/push and bind substantive review to the actual
head/path surface. Honor review quota windows. Pending review is not approval.
Track exact final non-WASM/Miri and honest separately waived WASM CI status.

## T7: ordinary main integration and root-serialized final merge

Refresh main normally, integrate only already merged origin/main into this owned
worktree with mandatory hooks, preserve both sides of real conflicts and all
protected owner functionality. Requalify affected integration, push/read back and
require actual final-head substantive review and required CI. Decimal work is
consumed only if already merged; do not edit its implementation or owner lanes.

Prepare concrete sibling squash notes and post them to issue/PR using stage-3.
Obtain root's explicit serial integration grant; verify main containment, clean
source/head/tree/notes and no live source reader. Integrate only through
`/home/paudley/stage/root/bin/ghprsq` from the exact owned worktree, with selected
.stage evidence only if the helper's actual capture contract requires it.
No hook bypass, alternative merger, manual base push or unauthorized lock use.
Verify signature, result tree equals qualified head, local/remote base/head/result
audit refs, both notes, notes archive bytes, PR MERGED, issue CLOSED and remote
branch deletion. Preserve all task evidence outside the owned worktree. Release
the lock and coordinate root-main FF/owned worktree/local-branch cleanup with root.

## Plan review and current status

Locally checked requirement completeness, additive API boundaries, old mode
compatibility, native test ownership, all maintainer comments and the source
homes above. Root approved the additive API/registry scope and the stop-checkpoint profile
advance with unchanged schedule/types/costs. This revision resolves the requested
transfer-inventory, wrapper/payload and uv-managed installed-proof wiring gaps.
Root must read back these revisions before implementation becomes actionable. Source is
clean at the captured base; no implementation or validation has begun. The
current blocker is deliberate authority/measurement sequencing, not a claimed
completed feature. Root approval and explicit window release are separate.


### paudley — 2026-10-05T19:19:45Z

T1's native mutable-dataset checkpoint is signed and pushed at `e0343ae16888bc983e8e184a85d871e241753261` (preceding mode commit `f7e1a811773542bb4b93155278bd858d1d010094`). Exact remote head readback matches.

`GraphExistenceMode::Implicit` preserves default construction and existing row-removal behavior. Explicit `RememberEmpty` retains empty named slots across mutation/snapshot/freeze until declaration withdrawal. The sole graph-creation home validates graph names and reports duplicate creation, while existing explicit declarations remain idempotent. Frozen and delta capability flags now describe declared empty graphs as well as graph rows. The public types are reexported through the existing Core/RDF homes; no mode bit or wire-format change is added.

Independent review caught an allocation in the delta capability probe. A native allocator witness ran before the repair and observed `[25936, 0, 25936, 136]` requested bytes for populated, withdrawn, retained, and restored snapshots; all independently checked graph-name/order/row/presence assertions passed. The shared effective graph iterator now answers existence directly, preserving ordered/deduplicated enumeration. The same witness passes with exactly `[0, 0, 0, 0]` bytes.

Current correction qualification: **58 native tests passed**, including graph modes, four allocator cases, named membership, canonical mapping, complete-state identities, and existing PACK goldens. Warning-denied affected Clippy, Core/RDF rustdoc, and normal signed commit hooks passed. The earlier 113-test T1 source receipt is retained separately; it is not relabelled as a run on this correction.

The corrected immutable benchmark executable is being prepared without running samples. Timing requires a coordinated exclusive window. This is a T1 task receipt: UPDATE operation semantics, stop/publication checkpoints, the installed Python surface, and full feature qualification are still active tasks on this branch. No feature-completion or PR-readiness claim is made.


### paudley — 2026-10-05T20:57:26Z

T1 checkpoint: original native benchmark evidence has been recovered without a rerun. The signed source remains `e0343ae16888bc983e8e184a85d871e241753261`; the original archived executable is bound to `33bc4f2f12e9c5dd874a88c3904d40217c763ab4`.

Session `32935` exited **0**: **15 baseline cases measured, zero failed**. All 15 saved estimate files, containing **1,050 authored raw samples**, are now archived with byte-for-byte comparison and matching SHA-256 hashes.

The actual compiled store is `/opt/.cargo/slots/7ccd5af5ca590a21/0/build/purrdf-bench`, derived from the executable's compiled `CARGO_TARGET_TMPDIR=/opt/.cargo/slots/7ccd5af5ca590a21/0/build/tmp` and the unchanged store rule. The initial read used the incorrect `/target/purrdf-bench` path and exited **2**; that failure is retained. **Absence of baseline files at the actual store before the run is NOT PROVEN**, because the original preflight checked the wrong paths.

There was no timing retry, fixture change, setting override, or acceptance-law change. The existing Rust harness retains its authored 100/10-sample cases, 3-second warmup, 5-second measurement target, 95% confidence, 10,000 bootstrap resamples, and 1% comparison law.

The corrected 15-case legacy comparison and eight additive graph-slot measurements are **NOT MET / not run**; timing remains **NOT GRANTED**. T2 UPDATE semantics, T3 stop/publication behavior, and T4 installed Python coverage are **NOT MET**. This checkpoint does not claim feature completion or PR readiness.

Durable local evidence: `/tmp/purrdf-400-mutable-baseline-recovery-addendum.json` (SHA-256 `7c78751a66f825ae371e3dd6eece1dba40ffdcd41a316af916da5b51e1e446ba`) records every source/archive path and hash, the original exits, and the preflight limitation. The remaining exact-path preparation is `/tmp/purrdf-400-mutable-after-exact-path-preflight.json` (SHA-256 `b517fd55d8cdc1f20fb25b086a3c4a16435833acbfe4331e92b35c778cd47b8b`). Original packet, preflight, and failure receipt are preserved unchanged.


### paudley — 2026-10-05T21:20:38Z

Measured T1 checkpoint: root independently audited the actual exits, all 76 produced/archive hashes, 38 schemas/raw arrays and 2,900 authored samples. Source work is now authorized; no new timing grant or retry is authorized.

Both authorized after phases completed with actual exit 0: legacy session `29104` (15 measured, zero execution failures) and additive session `7311` (8 measured, zero execution failures). All 38 before/after records contain the unchanged authored 2,900 raw samples and are archived with byte/hash equality.

These results compare the archived original `33bc4f2f12e9c5dd874a88c3904d40217c763ab4` against T1's clean signed `e0343ae16888bc983e8e184a85d871e241753261`, tree `a618a95949452f2443084a21b19af9a9d9ab04b3`. All 21 source hashes, three executables, original packet and preserved baseline/archive hashes are unchanged. They do not qualify final synchronized feature performance.

The existing native Rust harness calculated every legacy comparison using the unchanged 1% threshold, 95% confidence and 10,000 bootstrap resamples. Native verdicts: **8 improved, 4 no-change, 1 within-noise, 2 regressed**. An across-the-board non-regression claim is **NOT MET**. No causal or global performance benefit is asserted.

| Case | Original median | Corrected median | Native change | Native 95% change CI | Verdict |
| --- | ---: | ---: | ---: | --- | --- |
| `mut_build/cow_branch` | 44.072 ns | 42.114 ns | -4.4427% | [-6.4120%, -3.3237%] | Improved |
| `mut_build/simple_copy` | 208.54 µs | 219.26 µs | +5.1421% | [+2.1183%, +28.1657%] | REGRESSED |
| `mut_mutate/cow` | 229.26 µs | 220.34 µs | -3.8895% | [-11.5407%, +6.1985%] | No change |
| `mut_mutate/simple` | 420.8 µs | 336.74 µs | -19.9759% | [-22.2671%, -13.8972%] | Improved |
| `mut_query/cow_predicate_scan` | 255.28 µs | 214.64 µs | -15.9206% | [-18.0501%, -9.9742%] | Improved |
| `mut_query/simple_predicate_scan` | 11.206 µs | 10.748 µs | -4.0900% | [-6.0070%, -3.3422%] | Improved |
| `mut_freeze/cow_freeze` | 578.33 µs | 498.51 µs | -13.8028% | [-14.4708%, -11.3796%] | Improved |
| `mut_snapshot_graphs/snapshot_and_enumerate` | 6.6689 µs | 6.9519 µs | +4.2443% | [+2.3795%, +6.1208%] | REGRESSED |
| `mut_snapshot_graphs/annotated_base_small_drop` | 378.69 ns | 384.61 ns | +1.5631% | [+0.8492%, +2.5538%] | Within noise |
| `mut_snapshot_graphs/large_drop_repeated` | 1.5248 ms | 1.4776 ms | -3.0999% | [-3.7911%, -2.1052%] | Improved |
| `mut_declared_graph_removal/1000` | 159.17 µs | 148.52 µs | -6.6873% | [-29.2243%, -3.8996%] | Improved |
| `mut_declared_graph_removal/10000` | 2.129 ms | 1.791 ms | -15.8753% | [-22.0747%, -6.9798%] | Improved |
| `snapshot_with_delta/1000` | 409.12 µs | 443.4 µs | +8.3785% | [-5.6546%, +42.9545%] | No change |
| `snapshot_with_delta/20000` | 13.695 ms | 10.172 ms | -25.7244% | [-57.2935%, +24.5565%] | No change |
| `snapshot_with_delta/200000` | 131.57 ms | 131.32 ms | -0.1846% | [-10.1391%, +5.5029%] | No change |

The eight additive workloads have no original baseline; the following are standalone measurements, each retaining 100 samples. No improvement/regression classification is fabricated.

| Case | Median | 95% median CI | MAD |
| --- | ---: | --- | ---: |
| `mut_graph_slots/implicit/create` | 96.38 ns | [89.955 ns, 99.224 ns] | 9.5642 ns |
| `mut_graph_slots/implicit/last_row` | 196.71 ns | [194.95 ns, 198 ns] | 6.6867 ns |
| `mut_graph_slots/implicit/clear` | 144.12 µs | [143.52 µs, 145.4 µs] | 4.7016 µs |
| `mut_graph_slots/implicit/drop` | 142.71 µs | [139.86 µs, 146.52 µs] | 5.9271 µs |
| `mut_graph_slots/remember/create` | 118.46 ns | [117.84 ns, 119.44 ns] | 1.9981 ns |
| `mut_graph_slots/remember/last_row` | 144.91 ns | [144.2 ns, 146.08 ns] | 2.6182 ns |
| `mut_graph_slots/remember/clear` | 135.66 µs | [135.2 µs, 136.33 µs] | 1.5119 µs |
| `mut_graph_slots/remember/drop` | 136.7 µs | [135.9 µs, 138.25 µs] | 2.0381 µs |

The original baseline session `32935` exited 0. Its initial wrong-path raw read exited **2**, and that failure remains in the unchanged receipt. The actual compiled store is `/opt/.cargo/slots/7ccd5af5ca590a21/0/build/purrdf-bench`. Actual-store absence before the original baseline run is **NOT PROVEN**; the original preflight checked `/target/purrdf-bench`. Recovery copied the exact produced records without a timing rerun. No fixture, environment, sample, threshold, compiler or source override was made.

Evidence: `/tmp/purrdf-400-mutable-after-measurement-receipt.json` (SHA-256 `18c039bd5f38a8a3fd017697e63675ccf718724a349e9ddb3f25b75524a5fa93`) contains all paths, raw metadata/hashes, native comparisons, identities and exact exits. Raw archive root: `/tmp/purrdf-400-mutable-measurement-raw`. Legacy log SHA-256: `f68d1ea931123d85ad88dc041f36f39ae266aa3e58b3ba1f412440cbf8663158`; additive log SHA-256: `62f8d957df6067aa0c19bd9e09fcc3c8cf907b3d5417741a92ac56cdfd648ea5`.

T2 UPDATE semantics, T3 stop/publication behavior and T4 installed Python coverage remain **NOT MET**. Their approved implementation is active on this branch. Both performance regressions remain visible; this is not a ready-PR or full-feature acceptance claim.


### paudley — 2026-10-05T22:06:20Z

Native UPDATE selection is implemented and signed/pushed as `f590063b0f800a21eb417ae91c319cc0aa0c75fa` (tree `795f303e6ddfa6c925c1d6c489b454ec528511fb`). Both ordinary and governed doors accept the additive `QueryOptions::with_graph_existence` selection. RememberEmpty now distinguishes CREATE/CLEAR/DROP, validates transfer sources before touching destinations, and retains a successful empty LOAD destination. Implicit behavior and existing language/wire implementations remain unchanged.

The new native Rust target passed all **488 distinct cases**, including **312 transfers** (252 realizable distinct-target combinations and 60 self cases). The independent model checks complete rows and declarations, variable/constant GRAPH observations, same-request WHERE visibility, and whole-request rollback after a successful earlier operation. All 16 legacy graph-existence and 22 governed-UPDATE tests passed unchanged; the 41 UPDATE unit tests passed. The conformance run passed 44 manifest trials: 862 cases pass, five declared XFAIL, zero XPASS or unexpected failures. The unchanged governor corpus passed its 15 active cases, with its explicit regeneration case ignored.

Affected all-target warning-denied Clippy, warning-denied rustdoc and all six i18n gates passed. The mandatory signed commit hook passed, including the non-Rust ratchet with no changed non-Rust paths. The earlier formatting refusal and earlier compile/lint failures remain preserved; the formatting correction changed only the new CREATE test's wrapping. Root's independent source review confirmed that CREATE SILENT catches only native semantic admission/duplicate diagnostics; governor refusals are outside that catch.

Receipts: `/tmp/purrdf-400-t2-qualified-source.json` and its immutable final-format addendum `/tmp/purrdf-400-t2-final-source-addendum.json`; normal final commit/push logs `/tmp/purrdf-400-t2-commit-final.log` and `/tmp/purrdf-400-t2-push.log`.

Acceptance is **not complete**. Both measured T1 legacy regressions remain NOT MET (`mut_build/simple_copy` and `mut_snapshot_graphs/snapshot_and_enumerate`); no cause or performance benefit is claimed. T3's post-freeze publication/metadata-loop stop guards, T4's installed Python surface and Rust-owned proof, final integrated qualification and final-head review/CI remain required. No timing retry or new WASM semantic tests were run, and no PR-readiness claim is made.


### paudley — 2026-10-05T23:18:17Z

T3 is signed and pushed at `b050f7848130feffbbf5e9630be280660e3012d1` (tree `4e6c9cc262cb2e40eaacebfbbcd52d8ca9390c45`).

Governed UPDATE now checks the same request state after freezing its private branch and before the sole publication assignment. Bulk declaration withdrawal checks stop before each base or added declaration in one linear traversal. Direct native callback failure keeps the admitted prefix; whole UPDATE refusal discards its private branch and leaves the original handle and complete graph state unchanged.

Two real old-production witnesses failed before the repair: post-freeze publication and empty-only bulk withdrawal incorrectly returned Applied. The repaired neighboring refusal/success, SILENT cancellation/deadline and inclusive row-fuel cases pass. Actual native results: 6 core withdrawal/allocation tests, 26 governed UPDATE tests, 16 legacy graph tests, all 488 mode-matrix cases, 29 governor unit tests and the unchanged query-governor corpus (15 passed; its one explicit regeneration test ignored). The existing freeze gate verified all 26 vendored corpora. Affected all-target warning-denied Clippy and Core/eval rustdoc passed. The initial invalid cargo-doc invocation failed before compilation and remains recorded; corrected ordinary rustdoc invocations passed.

Profile version 12 identifies the added observable stop cut points: digest `a8d9fa11334a9cf4318e4ef8edaaf5d18032ae96c90778399335299824f1854a`. All schedule entries, indices, closed types, numeric costs and existing answer/spend/metered payloads remain unchanged. Only authored corpus profile metadata and its corresponding freeze/digest moved; corpus digest is `0a6f48072b0945d728f3e19d4d23dd60778259d4217277ec16bd3157fdf35554`.

Normal mandatory hooks and signature verification passed. Source/log identities are retained in `/tmp/purrdf-400-t3-qualified-source.json`; unsuccessful invocation and identity-reconciliation receipts remain retained separately.

Feature acceptance is **NOT MET**: the two measured T1 regressions (`mut_build/simple_copy`, `mut_snapshot_graphs/snapshot_and_enumerate`) remain unresolved and unchanged. T4 installed Python coverage and final integrated-source qualification are not completed; no ready-PR or performance claim is made.


