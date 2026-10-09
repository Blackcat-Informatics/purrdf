Closes #401

Repeated LOAD operations now give each resolved document fresh blank identities,
including nested triple terms and CDT List/Map references. ADD, COPY and MOVE
preserve ordinary, reifier and annotation record roles exactly in both
graph-existence modes.

The shared mutable core exposes additive typed record APIs while preserving
existing mutation, graph lifetime, indexed probe order and governor behavior.
Transfers select graph records before projecting owned values. This delivery
retains the current default; the planned breaking default change follows on the
landed foundation.

Validation: failing-first production regressions corrected; affected native
controls and ten portable production cases pass; the same ten cases execute and
pass on wasm32. Actual core/evaluator semver comparison passes against pre-change
main. Full `CI=1 make check` passes, including native workspace/consumer tests and
the real nested all-release `make wasm`. Normal commit hooks and push pass.

The first full run failed temporary-directory setup; correcting cache markers
only in task-owned Cargo output roots made the six owning controls and required
full retry pass. Source, guards and test assertions were unchanged; both runs
are retained in the qualification evidence.
