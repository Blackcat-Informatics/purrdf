# Task 4 consumer map

## Current source at `728d2d98d` (Task 4 commit `ae502bfc1`)

* **Representation and common view:** `crates/sparql-algebra/src/algebra.rs`
  keeps the explicit `GraphPattern::Project { variables: Vec<Variable> }` and
  adds `ProjectStar { variables: ProjectionVars }`. Its `projected_names()`
  returns the one `ProjectionNames` view defined with the persistent ordered
  snapshot in `projection.rs`; that view supplies ordered iteration, length,
  lookup and membership to both forms. Exhaustive downstream matches must
  adapt to this public enum addition.
* **Parser/scope ownership:** `parser.rs::VarScope` holds the ordered snapshot
  and contextual publication marker; `parser/machine.rs::sub_select_group`,
  `note_element_vars` and `finish_select` transfer the one-child scope and
  publish `ProjectStar` for simple stars, including trailing VALUES and
  RDFLib contextual MINUS. Deferred EXISTS uses the existing token stream's
  scope window. Grouped/expression projection remains explicit.
* **Serialization:** `serialize.rs` consumes `ProjectionNames` at the existing
  SELECT renderer. It emits ordinary `*` when the saved snapshot matches the
  inner visible names, and reconstructs a saved rewritten snapshot when it
  does not. Its one outer-position table and structural nested-star walk
  avoid serializing or preflighting every saved prefix at every level;
  shared-prefix, FILTER-wrapped and current-BIND MINUS controls are recorded
  in `T4-public-routes-diagnostic.md`.
* **Execution and remote consumers:** `crates/sparql-eval/src/eval.rs` sends
  both projection variants through `modifier.rs::eval_project_with` with a
  `ProjectionNames` view. `deferred_exists.rs`, `service_endpoints.rs`,
  `remote.rs`, `substitute.rs` and `expr.rs` keep projection order, scope and
  forwarded SERVICE identity through their existing paths. The public
  optimized `Dataset.query` entry is exercised in the package receipts.
* **Validation and ownership:** algebra `validate.rs`, `walk.rs`, `owned.rs`,
  `retained_size.rs` and `substitute.rs` handle both variants. Retained-size
  census counts reachable shared nodes across roots, and the final-owner
  allocator control confirms release after the last clone. Parser, algebra,
  evaluator and source-header controls are listed in `T4-implementation.md`.

This is a source/receipt map, not a whole-issue completion claim. The final
repository gate, high-u64 WASM identity fixture, historical #531 depth floors,
independent final review and integration remain open. The analysis below is
preserved as the **pre-implementation baseline**; its old line numbers and
"no qualifying commit" statement describe that earlier source only.

## Historical pre-implementation trace

`GraphPattern::Project.variables` is a public `Vec<Variable>` (`crates/sparql-algebra/src/algebra.rs:1067`). `Parser::finish_select` constructs a full vector for each `SELECT *` via `visible_variables_with_memory` (`parser/machine.rs:2823`), whose `note_vars` stops at an inner `Project` and visits its full vector (`parser.rs:4220`). `sub_select_group` then collects the full projected scope into a new `VarScope` (`parser/machine.rs:1710`), while enclosing groups add that scope to their own and `SelectStage::Where` visits it again. Nested fresh variables therefore drive quadratic visits and owned projection slots before evaluation.

`GraphPattern::Project` appears in 66 Rust files; at least 93 direct pattern sites refer to `variables`. Production consumers include algebra serialization and round-trip reconstruction, validation/scope/retained-size/owned-copy, evaluator execution, deferred EXISTS, substitution, property-function planning, remote SERVICE, statistics/governors and RDFLib projection. The parser's group scope and deferred EXISTS trail also need ownership changes to avoid re-copying the same growing prefix.

A late cumulative-slot cap leaves accepted depths quadratic. A linear lexical upper-bound check before parsing would stop the 8,000-level input but also leaves accepted fresh-variable doublings near 4x and can overestimate unrelated scopes. No cap-only change was retained. Meeting the accepted <3x requirement requires a shared/persistent projection representation and coordinated consumer migration. This task is open and has no qualifying commit, test or performance claim. The root owner requested serialization of that shared algebra/evaluator migration after S1's overlapping interface work settles.

Further read-only ownership trace: `Variable` already shares its spelling (`OwnedText`), so the quadratic resident cost is the repeated `Vec<Variable>` slot arrays and traversal, not duplicated name bytes. `VarScope` itself owns both `order: Vec<Variable>` and `seen: ScopeTable`; its `into_vec` transfers the order only after releasing the lookup table. `sub_select_group` reconstructs a new `VarScope` from the finished `Project`, then enclosing `note_element_vars` walks that scope and inserts every variable, and `SelectStage::Where` walks it again for deferred `EXISTS`. The nested one-child shape has an empty enclosing group at each handoff, where scope ownership can potentially be transferred instead of copied; general joins, `LATERAL`, `GRAPH`, `UNION`, and deferred `EXISTS` still require order and membership semantics. `finish_select` has already released the WHERE scope before constructing the star projection, so a compact projection alone would leave parser-level rewalks.

Candidate integration boundary after S1 settles: retain explicit projections as owned arrays, add a first-appearance ordered persistent star projection snapshot whose clone costs constant or logarithmic work, and carry the associated scope owner through SELECT/sub-SELECT handoff. Consumers should iterate in order and query length/membership through one public projection API. Measure parser scope visits separately from projection iteration/retained bytes; serializer, evaluator, deferred EXISTS and remote SERVICE must consume the same ordered snapshot without forcing a full list at every level. A `Project.variables` type change still touches many production constructors/matches, so this is design analysis, not an implementation or acceptance claim.
