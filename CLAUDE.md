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
* **PurRDF is NOT an ontology**: it mints no vocabulary IRIs. Vocabularies are
  caller-supplied configuration with no fabricated defaults (hard error or
  inactive feature when absent); test fixtures use `example.org`.
* **Perf changes need a bench**: extend the criterion benches rather than
  asserting a speedup.
* **`.deficiencies` is a last-resort emergency ledger, never an authorization surface.** Every entry below its marker is 100% unauthorized, 100% a bug, and means its originating issue or PR failed. It records critically undone work misrepresented to pass gates, misrepresented by an agent, or exposed when an agent is found fundamentally defective — literally a cry for help from a failing agent. The normal file contains only its notice and marker. Any entry blocks completion, PR creation, and merge of the work that produced it until the defect is verified and has a durable, visible remediation owner. Removing the entry does not resolve the bug or make the failed work successful.
