Consumers can now project the true default graph or an IRI- or blank-named graph
through `RdfDataset::project_graph`. Selection preserves independent graph slots
across assertions, reifier declarations and annotations, then emits only the
selected world into the default graph. The named-only method is removed and all
repository callers are migrated.

This removes the need for downstream code to reconstruct RDF 1.2 tables or
substitute a broader authored graph for the assembled default world.

Implements the native operation for #566. The paired downstream change is
[GMEOW #1837](https://github.com/Blackcat-Informatics/gmeow-ontology/pull/1837).
Keep the combined issue open until that consumer integration and its recorded
execution are complete; this native dependency lands first.

Validation: all 1373 core unit tests and 60 shared-view controls passed; one
existing large compaction control remains ignored by the ordinary inventory.
Six focused projection controls passed, including nested triple terms, shared
reifier identities, scoped blank graph names, empty worlds and invalid selectors.
Strict all-target Clippy passed for the kernel, RDF and SHACL crates. All 18 RDF
byte-determinism and eight SHACL graph-view controls also passed. The normal
signed commit hooks passed. All 49 hosted checks passed; four unselected
experimental lanes are skipped by their configured conditions. CodeRabbit
reviewed the final head with no actionable code finding and minimal assessed
risk. Its downstream-evidence warning is addressed by the published paired
GMEOW implementation; the actual GMEOW benchmark is still running.
