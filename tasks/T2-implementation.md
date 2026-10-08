# Task 2 consumer implementation

Status: SUCCESS — implementation, permitted qualification, independent review,
signed normal-hook commit, normal push/remote readback and exact issue update
readback complete. Stage remains separate from shipping source. Signed Task1 baseline:
e72660e8769af222387ecc4fd4f84ea9887e43a1.

## Implementation and provenance

Actual Graph, Dataset, ConjunctiveGraph, named-view and processor queries use the
typed contextual compiler. Existing native host query plumbing was factored once
into a statically typed QueryMode helper: ordinary mode and its associated
selection are unit values; contextual mode alone owns graph selection. Existing
configuration, relation, aggregate, term, GIL-detachment and result homes remain
shared. No dependency, feature, global mode, query rewriting, second evaluator,
or upstream implementation body/table was added. Existing frozen licensed vendor
data is read as data by the Rust witness; no vendor Python body is executed.

Contextual COW scope selection retains complete named metadata, with an O(1)
default-only fast path avoiding full surface materialization. Native set insertion
deduplicates union triples. Single-graph GRAPH refusal is boxed Apply policy and
raises only when visited. Validation and all clone/traits/planner homes retain
the marker; ordinary evaluator layout and dispatch have no added mode state.
Python duplicate columns remain positional, while named/get/asdict consistently
use the last label and preserve existing-unbound versus missing-key behavior.

Independent observations proved syntactic-group filter isolation was lost in
stacked filters. Only contextual parser GroupState folds its own filters in
source order through existing Expression::and; explicit nested braces stay
distinct and the existing VM is reused. Both failed driver/private-Project
guesses and all temporary probes were removed. A separate local-projection
EXISTS defect is fixed only at the non-isolated root filter expression view:
C union R, R winning duplicate names. Logical returned mapping remains R;
isolated filters borrow the original map without a new clone.

## Final source and artifacts

Rust15-path manifest: raw/t2-unit-mode-rust-start-sha256.log, SHA256
b558417ac39046a6ca70ebe02b087cda343961b05de1cc78fc023c6dd0651804.
Independent checks and writer checks after emissions/install/tests return0.
Final complete shipping manifest includes the new untracked test:
raw/t2-final-source-vocabulary-home-sha256.log.

Installed native module SHA256:
282264e4a45c28cad41f66dd7ec517d2f728f595360edd83b8affacf976979a4.
Locked UV file SHA256:
06275d43821c8351f89f88071af043ce4c4645fe3adcf2126b2f28f031cafc01.
Final new binding test SHA256:
9b547977e29cc2c0dde850cc4249e4e9784e8a00287c14b391a3808448d3ee49.
Actual importlib receipt binds the source native module, Python3.13.12 and genuine
RDFLib7.6.0 in the owned worktree environment. Root/sibling environments are not
mutated. See raw/t2-final-installed-identities.log and
raw/t2-final-module-lock-test-vocabulary-home-sha256.log.

## Actual allowed qualification

- cargo test --locked --release -p purrdf-sparql-algebra -p purrdf-sparql-eval
  --lib --test iterative_traits --test rdflib_contextual --test prebound_grouping
  --test exists_sep0007 --test request_substitution_feasibility: exit0,1909
  tests (464/3/1375/25/16/18/8), raw/t2-current-rust-core-controls.log.
- After the isolated-map allocation correction, the actual contextual target
  re-executed18/18, including241 permanent fixtures, six BNODE controls,
  configured callback counts/order/error/ASK cutoffs, grouping/governors,
  projection/empty/unbound neighbors, group braces and visited GRAPH operational
  errors. raw/t2-borrowed-filter-contextual-tests.log, exit0.
- Release clippy shipping algebra/eval/Python libs and affected selected test
  targets exit0: raw/t2-borrowed-filter-release-clippy.log. Final unit-mode Python
  lib release clippy exit0: raw/t2-unit-mode-release-clippy.log.
- Exact original frozen vendor Turtle/SPARQL witness executed by first-party
  Rust wrapper and native parser/compiler/evaluator: exit0, exact three columns
  and one expected row. raw/t2-exact-vendor-native-witness.log; input/wrapper
  hashes in raw/t2-native-vendor-witness-source-sha256.log.
- Owned binding install: uv run --locked maturin develop --locked, session21229
  terminal0, raw/t2-final-native-install.log. Actual optimized dev artifact is
  correctness evidence; distinct release emissions carry cost evidence.
- ONLY the14 irreducible Python cases ran after final installation. Latest
  uv run --locked pytest tests/test_contextual_mappings.py: exit0,14pass,
  raw/t2-final-python-only-bindings-vocabulary-home.log. Three expected
  ConjunctiveGraph deprecation warnings; no xfails/XPASS. The final test-only
  vocabulary-home correction uses the existing XSD namespace, unchanged terms.
- Query/interop/changelog/Chinese documentation updated through actual PO
  generator workflow; i18n/attribution/branding/link gates exit0, catalogue2927
  translated, zero fuzzy/untranslated/drift, raw/t2-current-doc-hygiene.log.

## Independent logical cost proof

tasks/T2-native-cost-review.md: MET for primary native Rust structural cost.
Actual signed-baseline and final candidate release artifacts are preserved,
including full false Parser::resume8k-line IR and assembly. Operations/control
flow match with only identifier/metadata/location-label normalization; native
parser frame remains1800. Main/library native dispatcher and caller match.
Boxed policy stays88/align8; graph144/expr64/query264/prepared368/node_ref16.
Untimed27-line layout/allocation/retention/peak/admission/prepare/reuse/prebinding/
configured-relation/resource receipts compare byte-identically, cmp0, shared SHA:
7cc55ec0ce0cdff2e1feade336b09b98153b496868cc523c8be02a8e1187dfc2.
See raw/t2-{base,candidate}-native-probe-run.log and paired SHA/cmp receipts.

Initial host const-bool capture retained extra scope storage and grew the frame;
it was rejected. Final unit-mode actual artifacts restore settled176/detached192
captures, erase identity Result adapters and scope storage, retain the direct
original12-argument caller and matching destructor. Host frame2264 <= baseline
2296; caller616 unchanged. Python emitted opt3/codegen1/native CPU stages have
no explicit LTO flag; native probes explicitly use actual ThinLTO. Neither is
mislabelled from the manifests' fat spelling. No clocks/timing benchmark/wasm
investigation is used. Unaffected frozen T1 core/SHACL evidence is explicitly
reused by the independent review, not rerun unnecessarily.

## User override and retained failures

Python may test only Python-specific behavior: receiver/import-shadow dispatch,
kwargs, PyO3 term carriers/exceptions/lifetime and Python row APIs. The new file's
header and per-test rationale identify these14 cases. No further broad pytest,
vendor semantic suite, Python semantic oracle or broad makecheck self-test lane
is run. Semantic controls live in Rust. Normal required hooks remain required.
Broad Python scoreboard rows are expressly historical snapshots outside the
untouched generated block; no unexecuted new full-suite count or vendor PASS.

All failed/superseded receipts remain visible. Initial full Python gate had one
real nested FILTER vendor failure. Two native guesses failed2vs1 and were removed
after independent root-cause diagnosis. Historical817/819 consumer controls
exposed separate projection positive/negative failures, repaired and tested in
Rust. Release all-target clippy found known prepared_execution debug-only helper
profile mismatch; selected affected release warning gate passed, no all-target
PASS claim. Final binding test and vocabulary-source receipts supersede earlier
test hashes while preserving earlier14-pass records.

## Owned cleanup

All task-owned build/test/inventory processes are terminal. The detached signed
baseline worktree was clean and removed normally after reviewers finished its
evidence; no sibling checkout/stash/environment was touched. Owned compiled IR
inventory tool removed; source and outputs retained. Temporary diagnostic edits
are absent. Managed shared Cargo/cache artifacts were not purged. Root's lossless
hash/byte-verified evidence packaging preserves oversized completed LLVM files;
no historical evidence was pruned. The active issue checkout remains necessary
for signed commit/publication and subsequent integration.

## Publication receipts

Signed Task2 commit: 56e636497c049debe2de5445389abf3e018d4783.
Tree: 33f75cadc2f87a89bec1230963ee68ec0262b99a.
Parent: e72660e8769af222387ecc4fd4f84ea9887e43a1.
Normal hooks and commit session41022 exited0; independent verify-commit exited0.
Normal push session64346 exited0. Exact ls-remote readback matches the full
Task2 commit on paudley/454-rdflib-shim-algebra-level-reassignment.
See raw/t2-signed-normal-hook-commit.log, t2-commit-identities.log,
t2-commit-signature-verified.log, t2-normal-push.log and
t2-remote-oid-readback.log.

Issue update: https://github.com/Blackcat-Informatics/purrdf/issues/454#issuecomment-6030643449.
Normal comment publication session84752 exited0; issue readback session56779
exited0. Exact body extraction with jq -j and cmp against the published body
file exited0, with no output newline alteration. See raw/t2-issue-update-body.md,
t2-issue-update-post.log, t2-issue-update-readback.json and
t2-issue-update-readback-exact.md. Final shipping status is clean; only separate
untracked .stage/ remains. All owned processes are terminal. PR creation and
final integration are separate from this completed task publication.
