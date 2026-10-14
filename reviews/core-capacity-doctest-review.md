# Core capacity doctest caller review

Verdict: PASS for the bounded source adaptation; executable documentation remains
UNRUN on this delta. This is not full-gate or whole-issue acceptance.

Reviewed the current two-file diff in `crates/rdf-core/src/graph_roles.rs` and
`crates/rdf-core/src/imports.rs`, the actual builder return signatures and the
retained `tasks/final-consolidated-full-gate.log`. The original seven failing
blocks are GraphRoleIndex (old line154), ImportMap (231), unanchored_imports
(405), unresolved_imports (526), declared_import_targets (798), imported_iris
(934), and unanchored_import_triples (995). These anchors refer to the failed
source, before added documentation lines.

Every changed `intern_iri` call now propagates its actual
`Result<TermId, StorageError>` before that ID reaches the next operation. Every
changed `push_quad` now propagates its actual `Result<(), StorageError>`. Each
example ends with the hidden explicit `Ok::<(),
purrdf_lex::allocation::StorageError>(())`, preserving the original typed cause
without formatting it or swallowing it. `purrdf-lex` is already a direct core
dependency; the error path is available to these executable examples without a
manifest change. The existing `freeze().expect("freeze")` boundary is unchanged
and does not require a conversion from its distinct diagnostic error.

The actual fixture terms, inserted triples, import classifications, graph-role
assertions and expected outputs remain identical. No example was ignored,
converted to compile-fail, or reduced to a weaker assertion. The diff changes
only documentation code blocks: no production body, helper, public API,
dependency or tooling behavior changes. No new duplicate function is added.

The edits directly address the retained compiler errors caused by passing
fallible interning results where TermId was required. Source review found no
material defect in this correction. Root reports edition2024 rustfmt and diff
checks PASS; this review did not rerun those commands or any build. The next
qualification is the affected original core doctests under standard Cargo when
the coordinated slot permits it. The other four failing doctest packages and
the separate segmented residency failure remain unresolved by this slice.
