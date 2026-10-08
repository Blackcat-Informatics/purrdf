# Task 3 implementation: canonical confirmations and opaque guard semantics

Parent source: f4873fabc. Status: implemented and focused checks passed; pending
independent review, normal commit/hooks/push by the root agent. No forge mutations
or commits were made by this implementer.

## Production changes

`Candidate` and assumed confirmations now share the numeric witness prefix and
the lexical fact tail. The preference remains `(saturated proof height, saturated
summed source heights, sorted source facts, authored rule, authored source facts)`.
The existing `source_heights` home supplies the arithmetic to both emission paths.
Lexical allocation still happens only on an exact numeric tie for candidates.

`Confirmation` retains the otherwise unobservable source-height sum with its owned
derivation. Each rule-local round buffer selects the best confirmation per row,
and merging parallel buffers compares competing proofs instead of concatenating
all firings. Schedule accumulation compares those winners across every group and
iteration in a layer, using owned `Fact` keys. Retraction cannot invalidate those
keys or source surfaces. Public `Derivation` layout and getters remain unchanged.

Ordinary prior-round/stratum admission remains unchanged. Assumed facts retain
their existing visibility and depth during the layer, and schedule/retraction
boundaries remain unchanged. Planner/calculus v2 already established by Task 1
remain the current delivery versions; authored hashes do not change here.

The guarded evaluator path was not rewritten or factorized. Source inspection
confirmed that production SHACL `Engine::evaluate` runs eager producers, assigns
with a newly minted prefix, mints fresh head blanks, and constructs/matches triple
terms. Lowering marks these binding-reading guards without certifying purity.
Consequently there is no memoization, existential reduction or per-row chain of
opaque guard stages. The callback's eager Vec allocation remains a later owned
memory-interface obligation, not a claim established by these tests.

## Runtime evidence

New scheduled public-entry tests verify all five witness components across a
single concurrent rule group, separately scheduled groups, reverse group order
and reverse seed insertion. An unrelated earlier row is retracted, and the
canonical proof surfaces remain valid. A separate iterating fixture discovers a
better confirmation in a later round while the same ordinary head retains its
original admission proof.

The existing caller-thread/stage-major test exercises ordinary and hybrid cyclic
plans through stratified and scheduled public Datalog entry points. New crossed
error tests ensure guard 0's later-row failure precedes guard 1's earlier-row
failure. A malformed callback row is a reached typed failure at that same stage.
These opaque callback controls use the actual Datalog seam: the public SHACL API
does not expose an arbitrary replacement for its private `Engine` evaluator.

New actual public SRL parsing/inference tests cover two disconnected positive
components with both assignment `BNODE()` and fresh head blanks, one mint per
logical solution, shared mint-state collision avoidance, and the empty-factor
neighbor. A computed assignment value absent from the model is correctly used in
negative probing. New public SHACL inference covers an OPTIONAL-backed opaque
Producer returning multiple CONSTRUCT rows with fresh template blanks; all rows
reach commit. Existing public `element_rules_lower_filters_assignments_blanks_and_triple_terms`
verifies actual production triple-term match/build and assignment/filter behavior.

Commands, each with at most 8 build jobs where applicable:

- `cargo test -p purrdf-datalog -j 8 --lib`: PASS, 328 tests, 0 failures.
- `cargo test -p purrdf-datalog --doc --locked --jobs 8`: PASS, 1 doc test.
- `cargo test -p purrdf-shapes -j 8 --test srl_language --test rules_engine --test rule_capacity_limits`:
  PASS, 23 + 26 + 6 tests, 0 failures.
- `cargo clippy -p purrdf-datalog --all-targets --locked --jobs 8 -- -D warnings`:
  PASS. One initial new-test single-character-pattern finding was repaired.
- `cargo clippy -p purrdf-shapes --test rules_engine --test srl_language --test rule_capacity_limits --locked --jobs 8 -- -D warnings`:
  PASS, warning-free.
- `cargo fmt --all --check` and `git diff --check`: PASS.

No full make check, whole-workspace test, wasm gate, host qualification or scaling
campaign was run here. Those remain assigned tasks of the approved delivery.
Task 4 credit/memory admission was not implemented here. No default, dependency,
semantic feature, public alternate engine or sibling worktree was changed.
