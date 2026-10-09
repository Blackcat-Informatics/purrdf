# Task2 independent implementation review

VERDICT: PASS

Current verdict: **PASS for the complete Task2 implementation and focused qualification**. Current five-source-path implementation retains the full Task2 contract. Final portable10, strict all-target Clippy, fmt and owning hygiene receipts each report actual exit0. Whole-issue completion remains pending Task3/4 additive semver, complete native/WASM qualification and normal delivery.

Reviewer performed source and evidence review only, with no builds or source/index/ref/forge changes. Implementation/check ownership remains with the Task2 writer. Scope is the changes after Task1 b0fc9e889 in core import/mutable selected export, evaluator Update, the portable Rust target registration and its tests.

There are no remaining Task2 implementation findings. Actual affected semantic scope is 663 existing/new selected controls plus ten portable cases: 43 Update units, 49 selected mutable units, CDT12, governed Update26, graph existence16, graph modes488, and core4/12/11/2. Both graph modes and other fixture combinations execute inside cases; these are not invented extra test counts.

## Contract adjudication

| Requirement | Source and meaningful evidence | Judgment |
|---|---|---|
| Exact ADD/COPY/MOVE roles in both modes | Source captures typed records before destination clear, rekeys only graph, uses typed insertion/removal without classifier. Portable controls exercise all three operations, default/named directions, ordinary/reifier/annotation collisions, orphan annotation, exact destination tables and unchanged source blank identities. Existing graph-mode and existence controls cover self/missing/empty neighbors. | PASS on final source and actual affected controls. |
| Distinct successful LOAD documents | A new `(label, BlankScope)` map is created after every successful resolution; first-occurrence traversal determines request-counter mints. Same cached Arc and IRI, separate requests and distinct IRIs sharing the Arc are covered through actual public engine calls. | PASS: portable-final executes all ten controls, including two distinct IRIs sharing the same cached Arc and the final independent collision fixtures. |
| Bare, nested triple and CDT identities | One shared iterative term fold and CDT rewrite home use the same pair map. Two scopes sharing one label stay distinct; repeated bare/nested triple/List/Map references co-refer within each document and separate across documents. Graph-only blanks are discarded; opaque literal bytes remain unchanged. There is no recursive replacement mapper or second blank-token scanner. Existing core/CDT traversal coverage remains applicable. | Established source and affected runtime coverage. |
| Mint capacity and destination collisions | Entire unique-pair batch capacity is checked before counter mutation; unit controls cover MAX/zero, MAX-1/one and insufficient two-pair capacity. Existing destination inventory includes unused/suppressed/CDT-only identities. Requested free prefix is preserved when the shared collision-prefix home returns None, and a concrete free-prefix positive neighbor remains mandatory. | Established source and runtime controls. |
| Actual physical charge and public atomicity | LOAD charges original physical rows before dedup; ADD N, COPY clear+N, MOVE clear+2N preserve existing cost home. Exact/one-below/zero neighbors use actual ChargePoint.cost(), not enum ordinals. Both modes retain original public Arc on refusal. Engine still freezes/assigns only after complete success; internal clear-before-charge ordering is unchanged. | Established semantic coverage. |
| Resolver, SILENT, stop, empty lifetime | Resolver absence/fetch/governed-error/post-return-stop ordering is unchanged. Failed/SILENT/canceled resolution creates no rows or declaration. Successful empty remembered LOAD declares only after stop admission; existing governor and graph-lifetime tests exercise these production seams. | Established source and affected caller coverage. |
| Selected payload and one-home efficiency | Mutable selected export resolves indexed IDs, filters physical rows before shared owned projection, and returns empty for missing bound terms. It does not own unrelated base literals before filtering. Cow borrowed ordinary lexical data reuses its original owned String. Original classifier, mint spelling, CDT decoder and import projection remain the shared homes. | Source findings corrected; selected core regression and integrations pass. |
| Additive/publication boundary | Task2 adds the Rust harness and corrects callers; no default-mode switch, dependency, feature, host-specific behavior or alternate freezer is introduced. | Source established. Actual semver, complete local/WASM and hosted delivery remain Task3/4 requirements. |

## Actual receipts read

All paths below are under `/opt/purrdf-401-qualification/logs/task2/` unless stated otherwise.

- `failing-before.log/.exit`: actual pre-fix failure of repeated LOAD, orphan annotation COPY and overlapping ordinary/annotation COPY; retained as failing-first evidence.
- `prefix-retry.log/.exit`: actual exit0, all ten public-engine portable controls PASS. Each applicable control loops both explicit graph modes; counts are cases, not mode-expanded claims.
- `update-units.log/.exit`: 43 PASS, no failures/ignored.
- `update-callers.log/.exit`: graph existence16, governed Update26, graph modes488 and CDT query/blank scope12 PASS, zero failures/ignored.
- `mutable-units.log/.exit`: 49 selected mutable units PASS; 1182 intentionally unselected lib tests are not counted as executed.
- `core-callers.log/.exit`: blank publication4, CDT identity12, graph existence11, import view2 PASS.
- `strict.log/.exit` and `strict-retry.log`: failed strict attempts retained, including a test-only assigning_clones lint. These remain failures, corrected by the later strict-final.log/.exit actual0.
- `portable-final.log/.exit`: actual0, current ten controls all PASS, with different-IRI cached-source and independently isolated unused/suppressed/CDT-only destination identities.
- `fmt-final.log/.exit` and `helpers-final.log/.exit`: actual0 for current `cargo fmt --all --check` and `make helpers-hygiene` respectively.

Task1 storage/validation/classification/replay/lifetime qualification is reused only where unchanged: full core1230 and its six integration suites, followed by the final per-role replay guard55/59 and strict/fmt PASS. Task2's changed selected-export API is additionally covered by the current mutable and four core caller runs above.

## Findings and corrections

The direct typed insertion diagnostic propagation is fixed; no IRI-only adapter erases the owning RdfDiagnostic. The ordinary literal Cow clone and whole-base owned projection findings are corrected in their existing homes. LOAD's missing free requested prefix was a real caller defect and is corrected without changing the shared destination-prefix law. Three original fuel expectations used schedule indices as costs; corrected controls preserve production prices and use the actual cost API. No required ceiling or semantic expectation was weakened.

Final source and receipts were read after the writer froze the production paths and the clone_from/fixture-only refinements. strict-final.exit, portable-final.exit, fmt-final.exit and helpers-final.exit each contain 0. portable-final reports ten PASS with zero failed/ignored/filtered; helpers-final includes the actual helper source census, not merely its self-tests. Layer/terminal/core hygiene were not independently run in this final helper command; normal commit hooks and the required Task3 full gate retain their applicable checks. Full semver/make check/make wasm/actual portable WASM qualification and publication are outside this Task2 gate, remain mandatory in Task3/4, and are not claimed complete here.
