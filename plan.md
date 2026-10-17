# One native graph projection

The owning contract is the complete issue body and acceptance list. Repository
goals, baseline and AGENTS.md govern the change. The existing named projection
already preserves independent graph slots across all three RDF 1.2 tables; retain
that behavior and its canonicalization controls rather than copying it downstream.
The unrelated REWORK_LIST.md in the main checkout remains untouched.

## Task 1: Replace the named-only operation

Replace `project_named_graph(&str)` with `project_graph(Option<&RdfTerm>)`.
`None` selects the true default graph. `Some` selects exactly one IRI- or
blank-named graph; invalid name kinds fail before row traversal. Every selected
row is emitted in the default graph. Use one graph-slot equality predicate for
ordinary assertions, reifiers and annotations. Migrate every repository caller
and Rust documentation link; delete the old method instead of adding an alias.
No dependency, schema, compiler-profile or wire-format change.

## Task 2: Prove isolation through the native operation

Extend the existing graph-projection tests with default and blank-named worlds
sharing a reifier, including a nested RDF 1.2 term and graph-local annotations.
Assert exact output content and graph erasure. Test absent and declared-empty
worlds and invalid graph-name rejection. Run the existing named isolation,
canonicalization and graph identity controls, affected-target Clippy and required
normal hooks. Tests construct only tiny synthetic RDF, never the GMEOW corpus.

## Task 3: Review and integrate the complete kernel change

Commit and push normally, open the issue-linked PR and obtain CodeRabbit feedback
where available. Record actual qualification and any missing independent review
without fabricating it. Merge only through ghprsq with selected Stage evidence.
The downstream GMEOW consumer then pins the merged native change, migrates its
callers and selects every benchmark world from the full production assembler.
Remove its authored-default shortcut; rerun the real default-world operation and
keep any genuine OWL admission refusal visible rather than falling back.

The separate combined stop/proof and shared-store/delta boundaries remain part of
the complete GMEOW engine migration. This kernel issue does not claim to finish it.
