# Persistent projection source checkpoint

Scope: projection.rs, retained_size.rs, parser.rs, serializer projection handling,
the evaluator modifier and shared projection view, and Project-only if-let paths.
This is an intermediate source disposition; the entire issue is not qualified.

The initial duplicate ProjectList representation has been removed in favor of
ProjectionNames. The newly added fork wrapper, whose body only cloned its receiver,
has also been removed. No second lexical parser is used: EXISTS and brace locations
are derived from the existing lexer tokens, which distinguish strings and IRIs
and omit comments. The owner reports the outer-EXISTS doubling control now passes;
its failed quadratic receipt must remain failed historical evidence.

The previous construction-delta byte counters could undercount a retained outer
root. They are now replaced by reachable Shared-node accounting with one visited
table across the complete algebra. The walk charges actual control allocation
layouts, deduplicates shared roots, and releases its traversal vector. Source
controls cover an outer-only root after the originating projection is dropped and
the same root retained through query Clone. Their latest terminal results remain
required; this review does not substitute for executed ownership/refusal controls.

The existing test support scope_candidates::visible initially recognized only
explicit Project and silently returned no names for ProjectStar. Root readback
now verifies its use of the existing projected_names view and common iterator;
the owner will run the original candidate controls. Production reasoning's
Project-only if-let occurs inside an explicit Project/Values arm and is paired
with a ProjectStar arm, so it does not establish a production omission.

Still open: the current focused evaluator answer test, final algebra rerun with
token/ownership/refusal controls, complete consumer and semantic audit, portable
installed execution, original allocation/variable-work acceptance and mandatory
full gates. No complete Task4 or issue PASS is recorded here.

## Subsequent material serializer review

Root identified a public AST rewrite that retains a three-column Star snapshot
while adding a fourth visible name beneath it. The previous serializer emitted
lexical *, exposing the extra column on reparse. The owner executed the concrete
unchanged regression: terminal101 before the fix, pass afterward. Root read the
actual saved-snapshot regression and corrected existing subselect path. Lexical
star now requires matching inner visibility; persistent nested one-append
snapshots compare shared trie prefixes, and other public rewrites use the existing
visibility computation before the explicit saved-name fallback. The failed
receipt remains in tasks/T4-rewritten-star-diagnostic.md.

The original 2k/4k/8k carrier allocation/byte controls remain passing. Root also
identified the saved-empty snapshot edge; the owner reports an executed passing
control after typed carrier refusal for a zero-column projection over visible
inner bindings, preserving unchanged empty wildcard rendering. Exact final
strict/portable evidence remains required. This does not mark the full issue
or every serializer state qualified.

The standard IntoIterator implementation delegates to the sole ProjectionIter
implementation. This required trait adapter adds no second traversal algorithm
and differs from the removed optional fork alias. The unchanged census passes
81 enforced jobs/118 distinct groups at its last observed source checkpoint;
later validation/lint changes still require final source applicability.

## Independent contextual-scope and MINUS carrier readback

Read-only review of the current `VarScope` generation in `parser.rs`, group/SELECT
handoff in `parser/machine.rs`, `ProjectStar` carrier in `serialize.rs`, and the
Task4 issue/plan/tests. No build or source change was made for this review.

The ordinary group scope still excludes a `MINUS` right operand. In RDFLib
mode, its right names are marked contextual in first-appearance order; ordinary
`contains` excludes them until `SELECT *` captures the saved ordered snapshot
and `publish_contextual` advances the generation. A later visible `BIND` of
the same name promotes its marker without adding a second projection slot.
Nested sub-SELECTs hand the snapshot-bearing scope back to the enclosing group;
`EXISTS` and `MINUS` in-scope frames remain isolated, and LATERAL consults only
the currently visible left names. These source paths show no material scope
regression in the inspected before/after-SELECT, BIND/LATERAL/EXISTS cases.

**Material carrier growth gap:** the local-BGP `MINUS` shortcut in
`serialize.rs` feeds all right-BGP names to
`ProjectionVars::is_appended_to(prefix, ...)`. That helper demands the exact
new suffix. `VarScope` deduplicates a name already in the saved prefix. Thus
an RDFLib nested chain whose level adds `MINUS { ?v0 <http://example.org/p>
?nN }`, after `?v0` was projected below, has snapshot suffix `?nN` but the
shortcut tests `?v0, ?nN`. It fails at every level. The generic fallback walks
ordinary visible variables, which omit contextual `MINUS`-right names, so it
cannot recognize the saved snapshot and emits each full explicit projection.
That recreates quadratic carrier bytes on an accepted nested fresh-variable
query. A non-BGP `MINUS` right operand (for example a `BIND` group) also lacks
the local shortcut and reaches the same fallback. Current 2k/4k/8k contextual
tests use distinct fresh `?mN, ?nN` right names, so they do not cover either
case. The issue's compact accepted-work/round-trip criterion is not yet shown
for these neighbors; retain Task4 as NOT MET until the existing serializer and
scope path handles them and the original doubling controls pass unchanged.

## Re-review of revised contextual MINUS serializer (2026-10-12)

The revised `VariableNames::for_pattern` records the outermost saved star's
first-position table once, with storage admitted through `Memory` and released
with the names owner. `ProjectionVars::has_prefix` checks the shared trie
without expanding it. The `Minus` branch now walks a non-BGP right operand via
the existing contextual visibility home, so the earlier `?v0` reused from an
older prefix and `FILTER`-wrapped right-side counterexamples are addressed in
source. The updated native controls exercise both through 8,000 levels; this
review did not run them independently. Saved public-rewrite snapshots still
fall back to an explicit saved list when their inner visible scope differs.

**One material accepted neighbor remains:** the position predicate skips only
right-side names whose outer rank is *before* `prefix.len()`. In the parser's
`Minus { left: Extend { inner: ProjectStar(prefix), variable: bound }, right }`
shape, a right operand may reuse `bound` itself. For each nested level, use
`BIND(i AS ?v_i) MINUS { ?v_i <http://example.org/p> ?n_i }`. The saved
snapshot appends `?v_i, ?n_i` to the prefix once. The serializer supplies
`?v_i` via `bound.into_iter()`, but the right walker also yields `?v_i`, whose
outer position equals `prefix.len()` and is therefore retained by the filter.
`is_appended_to` receives `?v_i, ?v_i, ?n_i`, fails, and the ordinary-visible
fallback excludes contextual `?n_i`, so it emits the full saved list at every
level. The current shared-name control reuses old `?v0`, whose rank is before
the prefix length; it does not cover this current-BIND overlap. The stated
accepted carrier growth criterion remains unproven for this shape. Correct the
existing local suffix computation and add this neighbor to the unchanged
doubling/round-trip controls before an optimized package or Task4 PASS claim.
