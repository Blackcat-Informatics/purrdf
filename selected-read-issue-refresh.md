# Issue #596: OWL DL: expose checked source-bound causal clash dependencies

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem and required behavior

Checked OWL DL clash replay currently establishes reachability by replaying the complete recorded application prefix. It resolves the terminal source individual, but that identity does not describe the actual causal source support. A consumer cannot safely attribute a global contradiction to local individuals from that identity. Unrelated earlier applications must not be reported as premises.

Deliver checked source-bound causal dependencies through the existing native replay and graph implementation. This is required by Blackcat-Informatics/gmeow-ontology#1803 before local class conclusions can delegate to the native owner. It is distinct from reusable general explanation sessions in #501 and from monotone source updates.

## Acceptance

- Expose typed source-aware concept, role and node views from the sealed consumer context. Preserve named terms, scoped blanks, literal datatype/language/direction, query assumptions, native normalization and proof-local generated identities. Numeric private identifiers are not source terms.
- Build the dependency graph during independent current-support replay using existing native graph operations. Slice the actual checked derivation backward from its terminal clash. Include asserted premises, normalized schema licenses, selected query/branch assumptions, equality causes and aliases, role paths and existential/inequality effects. Do not include unrelated prefix applications. Causal support need not be a globally minimal semantic justification.
- Schema-bound conflicts expose their actual checked bound derivations with the same source-aware views. Legacy missing current support must remain explicitly unsupported for this operation. Missing support cannot become an empty successful graph.
- Retain exact source/question/calculus binding, branch licenses, typed stopping/admission errors, and shared prepared-context ownership. No repeated solver search, second datatype/role/equality checker, source filtering or invented source vocabulary.
- Tests exercise disconnected inputs, derived memberships, normalized min/max bounds, inverse/transitive role paths, equality, generated witnesses, branching, scoped blanks and literals, wrong source/proof, missing support and stopping. Test actual public replay, not only conversion helpers. Native and packaged WASM acceptance are required.
- No semantic Cargo feature, shipping dependency, consumer namespace, compiler-profile change or host resource workaround. Normal hooks and ghprsq integration are required.

The downstream all-local-conclusion class integration and shared monotone world remain owned by the full GMEOW plan; this issue cannot alone establish their completion.


## Comments (7)

### paudley — 2026-10-17T22:22:02Z

# Checked native causal support

The full acceptance contract is the issue body, including source-aware resolution, precise checked causal dependencies, schema support, native and packaged WASM qualification. This work advances GMEOW 1803 Task 2; it does not finish shared world updates or all-local class delegation. Principles 4, 7, 17 and 18 govern the downstream boundary. Preserve native source ownership and one calculus. No local GMEOW corpus production, host probing/caps, hook bypass or alternative merge path is authorized.

## Task 1: Resolve checked evidence against its consumer source

Add borrowed typed views for every native concept constructor, roles, source/query terms, named/anonymous/reserved nodes and actual replayed witness facts. Expose them only after source-bound current-support replay succeeds. Preserve original term values and distinguish normalized native class structure from asserted RDF expression syntax. Reuse the existing XSD range AST. Tests must exercise actual public replay with independently prepared consumer source, source mismatch and legacy unsupported steps, plus complete structural variants. These views are an implementation component, not causal support or permission for local contradiction.

## Task 2: Record and slice the actual checked causal derivation

Instrument the existing native replay/graph operations with causal links for source assertions, query assumptions, normalized TBox/meta licenses, branch selections, equality aliases, role reachability and generated witness/distinctness effects. Reuse original semantic operations and match licenses; no duplicate role/datatype/equality solver. Slice backward from the checked clash and expose its schema derivations. Unrelated earlier prefix applications must be absent. No narrow PR or completion claim before this requirement is met. Resolve actual source expression provenance where parser normalization currently loses it.

## Task 3: Qualify complete source support

Exercise disconnected input, derived membership, min/max bounds, inverse/transitive paths, equality, witnesses, branches, scoped blanks/literals, tampering, missing support and stopping at the real public checker. Run affected native/scalar WASM tests, strict affected Clippy, normal hooks and actual packaged WASM. Retain all failures and distinguish pending hosted evidence. No elapsed-time optimization claim without measurements.

## Task 4: Review and merge

Inspect complete review surfaces, incorporate actionable CodeRabbit feedback and merge only through ghprsq with selected Stage evidence. No independent local dispatcher is available: local plan/task audits are explicitly self-reviews. No fabricated independent verdict. All issue requirements must be implemented before PR creation or issue closure. Preserve unrelated root REWORK_LIST.md and sibling worktrees.


### paudley — 2026-10-17T22:34:09Z

The borrowed source-view component for [native issue #596](https://github.com/Blackcat-Informatics/purrdf/issues/596) is implemented, committed through the normal hook and pushed on its native issue branch. It resolves checked concept/role/denotation witnesses and normalized schema bounds against the actual consumer context, preserving source scope, literal metadata, query markers and generated identities. It shares the existing replay and XSD owners; it does not copy the prepared knowledge base or create another semantic checker.

Native qualification passed: all ten new controls, the affected DL proof selection of 112 tests including those ten, and strict affected all-target Clippy. The measured terminal witness traversal allocated zero. This is not an elapsed-time speedup claim. Compilation failures and fixes are retained in the local process evidence. Local review is explicitly self-review, not independent approval.

The full issue remains in progress. Precise causal slicing, original normalization provenance, actual question-bound dependency graphs, equality/role/witness licenses and native/packaged WASM acceptance still need implementation and qualification. No PR is open for the source views alone, and this component does not authorize GMEOW local contradiction heads or complete class-engine delegation.


### paudley — 2026-10-17T22:49:03Z

Original normalization anchors are now retained for [native issue #596](https://github.com/Blackcat-Informatics/purrdf/issues/596). The native parser records the actual authored triple, axiom-node typing trigger or recursive source definition at each normalized inclusion. Query definitions and standard empty-role laws stay distinct from asserted source axioms. Checked schema views borrow the original term anchors, including the source expression's blank-node identity.

The empty-role helper also uses the shared schema insertion path, so later insertion invalidates both encoding and prepared bounds. Its control verifies recomputation and duplicate-law fixed-point behavior. There is no new solver, source filter, proof encoding, shipping dependency, compiler flag or resource cap.

Qualification passed: five new parser/ingress controls, the ten source-view controls, 184 proof-family regressions including those ten, all six public current-clash cases, and strict affected all-target Clippy. Normal commit verification passed. The initial test ownership errors and their repair through the native parser are retained in local process evidence. Review remains explicitly self-review.

This is a component of the full causal-support implementation. Original ordinals still need to survive absorption into clause/meta licenses, and complete source syntax, ABox/RBox support, key/equality derivations, selected role paths, witness distinctness and actual question-bound dependency slicing remain required. No causal-graph completion, packaged WASM qualification, downstream local-head integration or merge readiness is claimed. The full issue stays open; no narrow PR is being opened.


### paudley — 2026-10-17T23:06:45Z

Source links for [native issue #596](https://github.com/Blackcat-Informatics/purrdf/issues/596) now survive the actual native rewrite and compiler: antecedent/consequent and nominal-member splitting, partial absorption, range re-rooting, and internalization preserve their original inclusion ordinal. Dropped tautologies do not renumber the original sources, and equivalent/subclass source spellings remain distinct.

Prepared-context inspection borrows the original source anchors of emitted TBox clauses and internalized formulas. These are normalization licenses, not assertions of current membership or a complete causal support graph. One compact native-word tag now carries both provenance and object-domain scope; the redundant scope array and assignment path were removed. No new index-width cap, solver, proof encoding, dependency or compiler-policy change was introduced.

Current qualification passed: four new provenance controls, 187 proof-family cases, 11 clause cases, 14 absorption cases, all six public integration cases, the actual literal-versus-IRI domain-scope control, and strict affected all-target Clippy. Normal commit verification passed. Initial compiler/Clippy failures and their source fixes remain recorded. Review is explicitly self-review, not independent approval.

The full issue remains in progress. The next implementation is native replay dependency tracking using these preserved licenses, with irrelevant-prefix exclusion, selected body/path dependencies, equality and witness effects, original nested syntax and source ABox/RBox/key provenance. Native resource ownership, full question-bound controls, packaged WASM acceptance, external review and integration remain required. No narrower PR or downstream local-head completion is claimed.


### paudley — 2026-10-17T23:27:23Z

The actual unrelated-prefix control for [native issue #596](https://github.com/Blackcat-Informatics/purrdf/issues/596) now demonstrates a fully checked proof prefix containing an unrelated earlier class derivation. Its terminal witness mentions only the actual inhabitant. This confirms that reachability of a prefix is not precise causal attribution; the future dependency graph must exclude the unrelated application without filtering the input.

The native replay checker now validates the one recorded binding through its existing concept, denotation, role and successor predicates. It no longer enumerates alternative join/counting frames to rediscover that binding. Exact canonical nodes, variable introduction/width and counted ordering are preserved. Actual search keeps its original enumerator. The differential control covers all four body kinds and invalid/merged bindings; for its final six-of-ten selection, enumeration visits 210 frames while the direct checker uses fewer metered native work units. No elapsed-time or production-pipeline speedup is claimed.

Operational stopping and retained native storage refusal are checked before reporting malformed evidence. The stop-during-native-read control passed; no injected allocation-failure result is claimed. Current qualification passed both focused replay controls, 188 proof-family cases including the unrelated-prefix case, all six public cases and strict affected all-target Clippy. Normal commit verification passed. The initial missing test import failure and later qualification are retained. Review remains explicitly self-review.

The next work is the actual native replay dependency recorder, using the completed normalization links and selected-body boundary. Full source syntax, ABox/RBox/key/equality/path/witness support, native resource ownership, complete question-bound controls, packaged WASM acceptance and external review remain required. No causal-graph completion, downstream local-head integration or narrow PR is claimed.


### paudley — 2026-10-18T00:20:20Z

The shared ownership repair from #597 / PR #598 is now integrated into this causal-support branch through a normal merge with verified hooks. The combined code passed all 188 proof-family cases, four borrowed-context controls, six public current-clash cases and strict affected all-target Clippy.

This verifies that the source-view, normalization and recorded-frame work still consumes the actual independently prepared source through the shared carrier. It does not establish causal dependency slicing. The real disconnected-prefix control still demonstrates why the ordered application prefix is insufficient; actual dependency recording remains the next implementation. No narrow PR or full issue-completion claim is being made.


### paudley — 2026-10-18T01:12:32Z

The native mutation-journal component is implemented on this branch and passed seven focused controls, all 189 proof-family cases, six public current-clash cases, strict affected all-target Clippy, the full helper-hygiene target and normal commit hooks.

The original graph operations record prepared-source/query origins and checked application/head positions, including original fact transfers across equality, witness edges and inequalities, concrete value/domain changes and label withdrawals. Recording uses the native fallible memory and work/refusal owner. Cancellation, original storage refusal and unowned-write controls prevent a failed or unlicensed recording from being treated as successful evidence. The actual independently checked disconnected-prefix case retains identical frames/facts and identifies its unrelated write in the journal.

This is a mutation history, not a completed causal graph. It is requested through a private replay hook until selected body/path read licenses, immutable fact-parent links and backward slicing are implemented. Original syntax/ABox/RBox/key provenance, service-question graph binding and full native/packaged WASM acceptance remain required. The unrelated application is still in the raw history; no causal-exclusion pass, public graph, narrow PR, issue completion or downstream local-class delegation is claimed.


