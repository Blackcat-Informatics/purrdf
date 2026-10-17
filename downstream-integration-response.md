The downstream source is now reviewable in
[GMEOW PR 1837](https://github.com/Blackcat-Informatics/gmeow-ontology/pull/1837),
commit `7db11af84`.

Its [recorded consumer](https://github.com/Blackcat-Informatics/gmeow-ontology/blob/7db11af84/crates/pipeline/examples/recorded-class-consistency.rs)
unconditionally calls `assemble_object_level_edb` before selecting a world.
Selecting `default` produces `None`, which is passed to the native
`project_graph` operation. The authored-default helper and shortcut are deleted.
The full EDB is released after selection and before native reasoning; its
certificate and question-bound proof replay remain native. All named graph
callers and the canonical pipeline description are migrated as well.

The consumer type-check passed against the new API. The final dependency pin is
`af8936e6f22c70dcbc3f3dc1224be887aba51cc5`, including the reviewed source-receipt
renewal. Normal signed commit hooks and the normal push passed. Strict workspace
Clippy and the explicit optimized default-world operation are still running on
that final pin. No current default-world execution result or complete canonical
admission is claimed before that operation finishes.

This provides the concrete source evidence requested by the linked-issue
warning. The dependent GMEOW PR will be merged after this native PR; its complete
engine responsibility issue remains open.
