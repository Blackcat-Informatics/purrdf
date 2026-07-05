<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# Contributing to purrdf

Thanks for your interest in PurRDF. This repository holds the RDF 1.2 toolkit that
several downstream projects use as their data backbone: the Rust workspace, the
Python / WebAssembly / C bindings, the GTS transport engine, and the conformance
corpora that gate all of it. Issues and pull requests are welcome.

## Ways to contribute

- **Report a bug or request a feature** — open an issue with a minimal reproduction
  (ideally a small RDF document, SPARQL query, or failing conformance vector).
- **Fix a bug or add a feature** — open a pull request against `main`.
- **Improve the docs** — corrections and clarifications to the crate READMEs and
  [`docs/`](./docs/) are very welcome.

## Design constraints that PRs must respect

- **No Cargo features.** The workspace deliberately has zero feature flags and CI
  enforces it (`scripts/check-no-features.py`). PurRDF is a carrier: every consumer
  in every language must observe identical behavior. Do not add optionality; if a
  capability seems optional, discuss it in an issue first.
- **The kernel stays clean.** `purrdf-core` must not grow dependencies on oxigraph
  or PyO3 (enforced by `make rdf-core-hygiene`); `purrdf-iri`, `purrdf-xsd`, and
  `purrdf-events` stay zero-dependency.
- **Determinism.** Serializers and the GTS writer are byte-deterministic. A change
  that alters emitted bytes must update the affected goldens/vectors and explain why.
- **Conformance corpora are the contract.** W3C SPARQL, SHACL, RDFC-1.0 fixtures,
  and the frozen GTS vectors in [`vectors/`](./vectors/) must stay green. The GTS
  wire format itself is governed in
  [`gmeow-gts`](https://github.com/Blackcat-Informatics/gmeow-gts) — spec-level
  changes start there, not here.

## Development

```bash
make metadata   # regenerate + verify generated artifacts (loss matrices, queries)
make check      # fmt, build, tests, hygiene gates
make bench      # criterion benchmarks
make wasm-pkg   # build the ESM/wasm package
make capi-build # build libpurrdf via cargo-c
```

## Versioning & releases

**Pre-1.0 semver.** While the version is `0.x`, a **minor** bump may carry breaking
API changes; a **patch** bump is bugfix-only and API-compatible. The crates.io crate
suite, the PyPI `purrdf` package, and the npm `@blackcatinformatics/purrdf` package
share **one** workspace version and ship in lockstep — CI runs a version-coherence
check that fails if the three sources disagree.

**MSRV.** The workspace pins **stable** in `rust-toolchain.toml` and is nightly-free
by policy; the supported floor is `rust-version` in `Cargo.toml` (currently **1.96**),
which CI enforces with a dedicated MSRV job. Raising the MSRV is a notable, changelog-
recorded change that, pre-1.0, rides a minor bump.

**Changelog.** Write **conventional-commit messages** (`type(scope): summary`, e.g.
`feat(sparql): ...`, `fix(gts): ...`) — they feed the generated changelog, so their
wording becomes the release notes.

Release mechanics (tag-driven trusted publishing, provenance, SBOMs) live in
[`docs/RELEASE.md`](./docs/RELEASE.md).

## Before you open a pull request

- Run `make check` and make sure it is green.
- `cargo clippy --workspace --all-targets` must be warning-free — the workspace
  lint table (pedantic + nursery) is inherited by every crate, and CI denies
  warnings. Prefer fixing code over adding `#[allow]`; when an allow is genuinely
  right, scope it tightly and give it a reason.
- Every source file must carry an SPDX `MIT OR Apache-2.0` license header.
- Keep changes focused; describe **what** changed and **why** in the PR description.

## Licensing of contributions

Contributions to **purrdf** are accepted under **Apache-2.0 OR MIT** and, under the
project CLA, under terms that permit separate proprietary/commercial licensing.

By submitting a contribution you agree to license it under the terms above. For the
dual-licensing reservation to extend to your contribution, you agree to license it to
Blackcat Informatics® Inc. under terms that permit relicensing, including under
proprietary terms. A Contributor License Agreement (CLA) may be required before
substantial contributions are merged. See [`LICENSING.md`](./LICENSING.md) for the
full licensing scheme.

## Conduct

Be respectful and constructive — see [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md).
