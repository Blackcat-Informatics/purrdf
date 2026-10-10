<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Complete native prebinding rewrite handoff

`prebinding-rewrite-owner-draft.patch` is a standard unified Stage proposal against the current shipping worktree. It changes eight existing homes: evaluator `substitute.rs`, `expr.rs`, `workspace.rs`, `solution.rs`; algebra `algebra.rs`, `walk.rs`, `validate.rs`; and lexical `allocation.rs`. The four support `.rs` files and assembler are Stage construction inputs, not additional shipping modules. No shipping source, Git/index/ref, forge, compiler, build or test was changed/run by this support agent.

## Exact native query interfaces

All rewrite methods below use `Memory<'_, LexicalFrame>` and return the original `GroundFailure` (retained lexical diagnostic or typed operational `EvalError`). They operate on the original AST grant, not a post-construction estimate.

```rust
needs_assignment_join_with_memory(
    query: &Query, names: &[&str], memory: &mut Memory<'_, LexicalFrame>,
) -> Result<bool, GroundFailure>;

join_assignments_with_prebinding_with_memory(
    query: &mut Query, names: &[&str], memory: &mut Memory<'_, LexicalFrame>,
) -> Result<bool, GroundFailure>;

apply_shacl_prebinding_with_memory(
    query: Query, substitutions: Prebindings<'_>, memory: &mut Memory<'_, LexicalFrame>,
) -> Result<Query, GroundFailure>;

apply_shacl_probes_with_memory(
    query: Query, probes: &[(Variable, GroundTerm)], memory: &mut Memory<'_, LexicalFrame>,
) -> Result<Query, GroundFailure>;

bind_call_arguments_with_memory(
    call: &mut PropertyFunctionCall, values: &[(Variable, GroundTerm)],
    already: Option<&GraphPattern>, memory: &mut Memory<'_, LexicalFrame>,
) -> Result<Option<GraphPattern>, GroundFailure>;
```

The first API surveys assignment targets and VALUES context with an admitted work stack. It preserves the original joined/descent/EXISTS distinction; it does not compare or clone Queries. `false` means the cache may preserve its original source owner without constructing a replacement. The mutating API returns the same changed decision and uses checked fresh-name counters. A failed mutating call leaves only an unpublished private AST; the caller destroys it and retains the selected failure.

The patch adds these private adoption interfaces:

```rust
LexicalFrame::from_allocation(
    workspace: &WorkspaceCapability, allocation: WorkspaceAllocation, live_bytes: usize,
) -> LexicalFrame;
Memory::resume(storage: &mut S, live_bytes: usize) -> Memory<'_, S>;
```

The trusted caller takes the private `AdmittedQuery { query, allocation, live_bytes }`, adopts that exact original allocation into the frame, and resumes Memory at that exact original live total. Neither constructor allocates, resizes, admits a second lease or resets the live AST to zero. After successful mutation and destruction of construction scratch, read the surviving original total, end the Memory borrow, and publish the query with `frame.into_allocation()` and that total. Keep payload before its original grant in the private carrier. On failure select the original `GroundFailure`/workspace source cause before cleanup, destroy the AST, then release its frame/grant. Never publish a raw Query after dropping the frame.

`apply_shacl_prebinding_with_memory` builds fresh probes into that original Memory, borrows their entries throughout rewriting, destroys the probes before releasing their exact construction delta, and returns the rewritten Query. `apply_shacl_probes_with_memory` borrows an independently owned prepared probe carrier without cloning a probe Vec. The full SELECT/ASK/CONSTRUCT/DESCRIBE match remains exhaustive.

## Retained prepared probes and every failure

`PreparedProbes` holds ordered fields: ground entries, per-cell original child grants, entry-array grant, cell-grant-array grant. Its Drop clears all ground values, then the cell grants; automatic field destruction then frees both metadata arrays before either array grant. It acquires replacement arrays through the existing `WorkspaceCapability::reserve_vec` home and deep ground child copies through `clone_ground_with_memory`. Lexical leaves retain their intrinsic immutable `SharedText` owners. Unchanged IRI/typed-literal spellings and same-datatype leaves share their existing owners.

```rust
build_prepared_probes(
    probes: &mut PreparedProbes, substitutions: Prebindings<'_>, workspace: &WorkspaceCapability,
) -> Result<(), GroundFailure>;
PreparedProbes::entries(&self) -> &[(Variable, GroundTerm)];
PreparedProbes::reset_after_failure(&mut self, workspace: &WorkspaceCapability);

ParameterValue::from_id_admitted<D: DatasetView>(
    dataset: &D, id: D::Id, workspace: &WorkspaceCapability,
    source_error: impl FnMut(D::ReadError) -> GroundFailure,
) -> Result<ParameterValue, GroundFailure>;
```

`build_prepared_probes` selects its failure before resetting the carrier. For **every failed bounded preparation or run**, including lexical failure, plan/compiler failure, typed source refusal, governor/stack/allocator refusal and evaluation failure, the enclosing caller must likewise select the terminal original cause first, then call `reset_after_failure`. Do this even when failure occurs before calling the builder. Cleanup does not allocate, resize a grant, format a diagnostic, or replace the selected cause. It detects original bounded array/cell guards as well as the current bounded capability, so supplying a resident capability cannot shed or retain an earlier bounded payload incorrectly. Only an entirely resident carrier under a resident capability preserves its empty spare arrays for the existing resident reuse law. Do not use `clear()` as bounded terminal cleanup.

The id door stores `ParameterValue::AdmittedGround(SharedWorkspace<AdmittedGroundTerm>)`; ordinary clones retain the original child grant shallowly. Resolve typed dataset failures through the enclosing workspace source-error latch, rather than a raw resident diagnostic. Update any caller's exhaustive ParameterValue match to include the new admitted variant.

## Native implementation coverage

- Pushdown preserves OPTIONAL/MINUS/projection/group/remote scope law, original order, narrowed probe membership and binding restoration. Narrowed probes use a shallow `SharedWorkspace<PreparedProbes>` owner instead of a deep Rc/Vec copy. Each actual replacement buffer, ground child and graph child is admitted before its factory.
- SHACL substitution preserves its intentional divergence from SEP-0007. Original graph postorder frames, own-expression slots, EXISTS bodies, driver construction, carried columns, group keys and stand-in names use original Memory or existing admitted scratch/schema owners. Aggregate arguments and FOLD keys mutate fixed existing slots through `AggregateExpression::expressions_mut`; no take/collect/reallocate roundtrip changes aggregate invariants.
- Assignment and MINUS passes share an iterative frame walk: own EXISTS bodies in source order, node effect, then original last-child-first descent/restoration. Nested EXISTS levels do not recurse on the machine stack. Fresh identity increment is checked before constructing a name. The existing SeedColumns bitmap remains an optimization with its original wide fallback, never an admission ceiling.
- The existing expression mention collector is the single `collect_vars_admitted` body, returning `VarSchema`. `pattern_all_vars_admitted`, `pattern_vars_outside_admitted` and `expr_vars_admitted` retain widened EXISTS, nonnarrowing Project and exact excluded-SERVICE-endpoint behavior; they do not substitute syntactic certainty. Native stacks, schema columns and membership tables use their current admitted homes. Existing resident collectors delegate this same body.
- Shared algebra mutable-expression and term-variable walkers expose fallible visitor/storage results at their existing homes. They retain source ordering, repeated variable occurrences and quoted predicate/subject/object order. Current `AdmittedVec`/`SchemaBuilder` extension additions reuse the existing growth bodies.

## Shipping integration obligations

The root cache/preparation packet and sole writer own production caller wiring. This support proposal does not claim that those callers are already migrated.

1. Root native preparation/cache: use the readonly assignment decision before choosing original-source preservation versus `Query::clone_with_memory`, and run all query mutations under adopted/resumed original Memory. Preserve the exact old source grant while its tree is live; destroy that tree before releasing its original layout total. No raw Query Eq/clone or resident substitution is a bounded preparation door.
2. Writer `PreparedExecution`: replace `Vec<(Variable, GroundTerm)>` probes with `PreparedProbes`, use `build_prepared_probes`, borrow `entries()`, use the admitted id door, and call `reset_after_failure` on every failed bounded preparation/run outcome. Any separately retained binding buffers need their enclosing native caller's own failure cleanup; they are not covered by a probe guard.
3. Actual correlated `expr.rs` callers of `bind_call_arguments` (current shipping calls near 3275/3923, subject to concurrent line drift): call `bind_call_arguments_with_memory` under their actual owned expression/AST frame. Do not wrap the resident constructor and price its result afterward. Native Query/GraphPattern/Expression clone APIs from the numeric packet supply their copied original AST.
4. Any bounded Rdflib transform still using the re-exported raw mutable-expression walk must call algebra `for_each_expression_mut_with_memory`; the raw resident adapter stays for the existing public resident path. No bounded transformation should reach resident `apply_shacl_*`, `build_probes[_into]`, `bind_call_arguments`, or raw mentioned-variable collection.
5. Apply the numeric `native-algebra-clone-owner-draft.patch` and root allocation-free algebra destructor changes with this unit before qualifying failures/deep inputs. This patch relies on their original native clone/drop bodies, including destruction of partially rewritten graph/expression children after refusal. Ground/TriplePattern allocation-free destruction is already a dependency of the integrated grounding/request unit.

## Evidence and qualification still unrun

Standard dry-run applicability and rustfmt syntax parsing are the only support checks. They do not establish Rust type checking, lint acceptance, semantic parity, physical closure or complete #508 acceptance. The sole writer must qualify the integrated source through the managed lane.

The patch preserves the existing independent generated recursive oracles and all their semantic assertions. It adds five evaluator fixtures at the existing real non-latching reservation test home and one neutral Memory fixture: four-form original grant adoption/drop; first buffer refusal retaining exact original cause without clones; prepared spelling reuse followed by complete bounded lexical-failure cleanup; resident lexical-failure array reuse; checked assignment identity overflow; and resumption preserving an old buffer during refused replacement overlap. All are **UNRUN**.

Required real-entry acceptance: direct and prepared four-form requests, owned/borrowed/id bindings, valid/malformed lexical values, bounded success/refusal during narrow probes, OPTIONAL/MINUS/EXISTS/APPLY/LATERAL/group/FOLD/project/VALUES/bag/quoted terms and deep stacks; source/allocator/governor failure precedence; cache source identity when assignment changes versus no change; repeated prepared success and every failure outcome leaving no previous run's fresh query charges; retained diagnostic/result owner clones remaining covered after context destruction. Run the unchanged semantic oracle corpus as part of the grouped qualification. No subset, capacity multiplier, opaque refusal or post-census price substitutes for these paths.
