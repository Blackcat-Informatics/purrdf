# Task 2 implementation and consumer qualification

Status: SUCCESS. Complete assigned Task2 source and qualification are delivered.
No commit, push, forge message or independent final verdict is claimed by this
implementation handoff; those remain the parent's authorized workflow actions.

Worktree: /home/paudley/Active/purrdf/.worktrees/457-paged-tier-an-lsm-style-stack-of-sealed
Branch: paudley/457-paged-tier-an-lsm-style-stack-of-sealed
Starting/current HEAD: 331c44e93aa6bcf8434dd027c4e0aae2b5b939b9.
Base synchronization parent: b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac.
Plan: SHA-256 ce1721dae29798cf2657336171b9981a0a1d86eec1ef0682392fc7379081ea29.
Source file manifest: raw/T2-source-sha256.txt; final hashes must be refreshed after
any qualification repair. Core implementation and contract source remain unchanged.
Final staged candidate tree: 13496228db0e5ec79074acad9020cac5aab7556e.
Full staged five-file patch: raw/T2-source.patch, SHA-256
040958eab3e246bd0b1d15f3ba51e39bf07d0f8d6a2cc49aefb86c572ab42a84.
The parent staged exactly those files after verifying the manifest. The writer
did not stage, commit or rewrite source history. Final hashes were rechecked
against all five live files and matched; source and Cargo ownership are released.

## Real production callers and independent inputs

crates/sparql-eval/tests/paged_stack_query.rs invokes the public native engine's
query_fallible_view and query_prepared_fallible_view directly on fresh views from
retained PagedStackSnapshot values. Each ordinary expected surface is a separate
BTreeSet<QuadValues> mutated independently; RdfDatasetBuilder constructs its eager
input, never a drained stack. Explicit expected RDF 1.2 reifier/annotation tables
use public typed builder methods. Result comparisons preserve variable order,
exact ordered solution bags and booleans, and compare actual PackBuilder graph
carrier bytes for CONSTRUCT. Typed stream counts guard classifications that bag
parity alone cannot establish. No eager scratch view replaces the stack caller.

The ordinary matrix covers 19 query texts: cross-generation BGP/FILTER, OPTIONAL,
UNION bag and DISTINCT, MINUS/NOT EXISTS, aggregation/subquery, paths, named graph
variable/constant/empty membership, ASK chronology, CONSTRUCT, ordered slices,
and delta-only populated/declaration-only graphs. It runs before changes, after
retraction/sealing, after reinsertion into a resident head, after head sealing,
after delta graph withdrawal and again over retained earlier readers. Independent
bases deliberately disagree at dictionary ordinal zero and each has page zero.
Repeated physical facts count once in the logical set; a four-edge nonempty
witness is asserted. Two bounds and distinct physical histories compare every
ordered compacted page's actual bytes to the independent eager input.

The generated sequence applies 32 deterministic value mutations and compares
five real guarded query families after each snapshot. A preserved anchor cycle
asserts an exact nonempty property-path answer at every step. Readers retained
at five sealing boundaries are rerun later and compared by actual compacted
page bytes at bounds one/five. A fully retracted single-row history exercises
empty consumers and zero output pages.

The typed fixture preserves same-label blanks in distinct scopes, nested triples,
directional language literals, reifiers and annotations in named graphs. Removing
the last original declaration produces independent expected ordinary rows; a
later head declaration in a different graph promotes only the matching older
ordinary row. Both head and sealed promotion states use the real evaluator,
assert typed counts and compare eager canonical page bytes. The native legitimate
ordinary/virtual-reifier overlap remains present in both tables and reaches the
same consumer bag/aggregate as a separately constructed native input.

Controllable counted providers are armed only after their successful initial
seal. Aggregate receipts assert source-qualified page-zero addresses, head plus
all sources' exact bytes/pages, first-request order, inclusive ceilings, zero
and one-below limits, and no materialization of refused sources. A repeated
ordinary/prepared query on the same view preserves the entire receipt and read
counters. Provider refusal, provider cancellation, deadline, explicit invalid
data and actual changed content all remain Operational with no diagnostic or
partial answer. Sticky faults gate metadata/head/constant consumers and compact
returns no artifact. Generation and page-count drift in source ordinal one of a
two-source vector, including a zero-page source, refuse both constants-only and
head-only queries without requesting content.

crates/sparql-eval/examples/paged_stack.rs is a runnable public consumer. It builds
independent layers, removes/replaces a name, seals a batch, retracts/reinserts an
edge, retains an old reader, executes ordinary/prepared fallible joins, verifies
cache receipts, prints asserted observations, and folds against independently
constructed eager input with ordered carrier identity. Shared input/carrier
helpers live once in examples/support/paged_stack.rs and are used by the tests.

The supported RDF reexport module now exposes every new stack/canonical symbol.
The existing umbrella smoke constructs, mutates, snapshots, queries and folds
through purrdf's root and sparql module, with no direct consumer core dependency.
No dependency, feature, profile, generated artifact or helper exemption was added.

## Executed checks so far

Every Cargo command uses CARGO_BUILD_JOBS=2 and the explicit worktree above.
The governed floating nightly is rustc 1.100.0-nightly, commit
4b6d04e706108ccfeafe2547fbe857dfe8972bad, LLVM 23.1.1.
Native tests use the repository test profile: opt-level=3, debug-assertions and
overflow-checks enabled, debug=0. The example uses normal dev (opt-level=3,
debug-assertions/overflow-checks enabled, debug=1); no profile override is used.

| Command | Exit/result | Evidence |
| --- | --- | --- |
| cargo test --locked -p purrdf-sparql-eval --test paged_stack_query --test fallible_query --test paged_query_e2e --test delta_view | 0; 32 tests passed, zero failures or ignored; new target 7 pass, fallible17, paged7, delta1; no cases filtered | raw/T2-evaluator-qualified.log |
| cargo run --locked -p purrdf-sparql-eval --example paged_stack | 0; Bob retained reader: 2pages/202bytes; Robert current: 4pages/1088bytes; compacted1page, actual ordered carrier identity asserted | raw/T2-example.log |
| cargo test --locked -p purrdf --lib facade_exposes_the_completed_umbrella | 0; one intended test passed, 43 unrelated lib tests filtered | raw/T2-facade.log |
| cargo clippy --locked -p purrdf-core -p purrdf-sparql-eval -p purrdf-rdf -p purrdf --all-targets -- -D warnings | 0; all affected targets checked without warnings | raw/T2-clippy-qualified.log |
| cargo test --locked -p purrdf-sparql-eval --test paged_stack_query | 0; seven tests passed, zero failures/ignored/filtered after mechanical clippy repairs | raw/T2-new-final.log |
| rustfmt --edition 2024 --check over the five Task2 Rust files | 0; final files formatted | raw/T2-format.log |
| git diff --check | 0; no whitespace errors | raw/T2-whitespace.log |
| cargo build --locked --release --target wasm32-unknown-unknown --lib -p purrdf-core -p purrdf-sparql-eval -p purrdf-rdf -p purrdf | 0; all four selected libraries and transitive public-facade dependencies built in governed release profile; 3m00s | raw/T2-wasm-qualified.log |
| make helpers-hygiene | 0; census/self-tests: 81 enforced jobs, 23 reasoned variants, no open copies, 91 current distinct rows, 1833 files; 80 unique prefix-free hash domains | raw/T2-helpers-qualified.log |
| make build-profile-hygiene | 0; self-test; all 984 resolved units across two gate invocations (42 workspace members) actually use opt-level3 with debug assertions/overflow checks enabled | raw/T2-profiles-qualified.log |
| git diff --cached --check | 0; all five staged files have no whitespace errors | raw/T2-staged-whitespace.log |
| sha256sum -c raw/T1-source-sha256.txt (full stage-prefixed path) | 0; every one of the 11 Task1 production/test/doc input hashes still matches | raw/T2-core-applicability.log |

Example source/helper hashes remain unchanged since its execution. Later changes
were only additional evaluator-test coverage and therefore do not invalidate the
executed example. The final evaluator qualification includes all preliminary
reviewer coverage repairs. Core Task1 qualification remains reusable only for
its unchanged implementation inputs; it does not replace this evaluator run.

Historical raw/T2-new-first.log had 5pass/1fail: a fixture assertion discovered
both dictionaries began with alice. The independent second input now begins bob;
the witness assertion was preserved, not weakened. raw/T2-new-second.log passed
all six tests before the additional reviewer coverage cases. Neither historical
run replaces the final seven-case qualification. raw/T2-clippy-first.log found
two assert_is_empty diagnostics and one redundant clone. Empty collections now
use exact zero-length array equality (with better mismatch output), and the blank
value moves after its last use. No assertion or production behavior was weakened.
The new target reran after these changes. The other 25 executed consumer tests,
example and facade source/dependency inputs did not change, so their earlier
executions remain applicable. raw/T2-toolchain.txt captures the compiler; the
home Cargo build config supplies -D warnings without a target replacement.
An initial large shell heredoc
write was rejected by the policy parser before mutation; apply_patch performed
the source writes. There was no approval, hook or verification bypass.

## Final identity and completion boundary

| Task2 file | Final SHA-256 |
| --- | --- |
| crates/sparql-eval/tests/paged_stack_query.rs | e362206826c0ac977be6ec2269caefb5bff27a09f7452e088c3303ba2095ebce |
| crates/sparql-eval/examples/paged_stack.rs | 765580136161854f9586d3314c3ccb7455eb2f2b80530cfc4aa4eb53d3d80ff0 |
| crates/sparql-eval/examples/support/paged_stack.rs | e5df7a0504d4f2c517fc308dca5402f0fac0fd8724e7ab73f8f30488726837f7 |
| crates/rdf/src/lib.rs | 1b0f9cad6ac0fb83724c5272db77dd7d556d235fd7f19ca49d92e9587fc6eefc |
| crates/purrdf/src/lib.rs | 7035d22faa1e23ab170f1a4b5d429395736e35cdae7667383fc31b07d025e703 |

No required Task2 implementation or assigned qualification remains open. The
new target executed seven tests on the final spelling. The other 25 affected
tests executed earlier on the same production and unchanged test inputs; only
three mechanical lint fixes to the new test came afterward. The unchanged
example/facade hashes bind their successful executions. Core input hashes all
match the reviewed Task1 manifest. Profiles were checked against Cargo's resolved
unit graph rather than merely inferred from Cargo.toml. No production model,
service lifecycle action, dependency/configuration change or sibling process
interference occurred. All owned Cargo/make invocations are complete.

Wasm evidence proves release compilation, not wasm runtime execution. This task
does not establish the full workspace test/conformance gate, hosted CI, final
independent task/completion review, hooks/signing, publication or integration.
Required independent review and the parent's Stage workflow continue from this
stable source/qualification handoff. No whole-issue completion or merge is claimed.
