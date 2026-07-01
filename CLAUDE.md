# Claude Code Instructions (CLAUDE.md)

Read [AGENTS.md](./AGENTS.md) — it is the canonical contract for this repository
(crate map, hard constraints, commands, performance discipline, brand rules).

Quick orientation:

* **Gate before you claim done**: `make check` (fmt + clippy + build + tests +
  hygiene). Clippy runs the workspace pedantic/nursery lint table and must be
  warning-free.
* **Never**: add Cargo features; hand-edit `generated/` or `vectors/`; pin a
  dependency version inside a member crate (root `[workspace.dependencies]`
  only); introduce nondeterminism into serializers or the GTS writer; add
  oxigraph/PyO3 anywhere near `purrdf-core`; break the wasm32 build (every
  release crate must stay `wasm32-unknown-unknown`-clean — `make wasm`).
* **Naming**: the project is **PurRDF** in prose, `purrdf` in identifiers.
* **Perf changes need a bench**: extend the criterion benches rather than
  asserting a speedup.
