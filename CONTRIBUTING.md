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
make doctor     # what this machine actually enforces (run when a gate prints SKIP)
make metadata   # regenerate + verify generated artifacts (loss matrices, queries)
make check      # fmt, build, tests, hygiene gates
make bench      # criterion benchmarks
make wasm-pkg   # build the ESM/wasm package
make capi-build # build libpurrdf via cargo-c
```

`make check` is the gate, but it is not the whole gate: **`make conformance` and
the rustdoc lane run separately**, so adding a test can leave `make check` green
while `make conformance` goes red. Run both before claiming a change is done.

### A SKIP is not a pass

Several gates degrade to `SKIP` when a tool is missing locally and hard-fail in
CI instead. `make doctor` prints which ones are inert on your machine and why.
The distinction that matters most: if `rustup` is not on `PATH`, then
`rust-toolchain.toml`'s dated-nightly pin is **not enforced locally at all** —
`cargo` resolves to whatever is on `PATH` — and `make wasm` can neither detect
nor install the wasm32 target, so it skips silently. That is a different
situation from "the target is not installed", and `make doctor` says which one
you are in.

### Benchmarks

Benches are **report-only** and never gate; PurRDF asserts no speedups (see
`docs/BENCHMARKS.md`). They run weekly and on demand from
`.github/workflows/benchmarks.yaml`, not on every push.

`cargo bench --workspace` works. It did not until recently: `bindings/python`
sets `test = false` on its lib because a PyO3 `extension-module` cannot link as
an executable (it leaves the CPython API unresolved for the interpreter to supply
at `dlopen` time), but `test = false` does **not** disable the auto-generated
BENCH target — so `cargo bench --workspace` failed to link with ~70 undefined
`pyo3-ffi` symbols while `cargo test --workspace` was green. It is now
`bench = false` as well. CI never saw it because the benchmark workflow names
crates individually.

`[profile.bench]` inherits `lto = "fat"` and `codegen-units = 1` from
`[profile.release]`, so a full bench build is slow; naming the crate you care
about is usually what you want:

```bash
cargo bench -p purrdf-sparql-eval --no-run --locked
```

### Reclaiming disk from stale cargo build directories

If this machine sets `[unstable] build-dir-new-layout` in `~/.cargo/config.toml`,
cargo keys each build directory on a hash of the **workspace path**, so every
worktree gets its own tree and those trees **outlive the worktree by design** —
nothing cleans them up when the worktree is deleted. On a checkout with heavy
branch or worktree churn they accumulate without bound and can fill the disk.

`scripts/sweep-cargo-build-dirs.sh` maps each hash directory back to the
workspace that created it and reports the ones whose workspace no longer exists.
It prints by default and deletes only with `--delete`:

```bash
bash scripts/sweep-cargo-build-dirs.sh            # report only
bash scripts/sweep-cargo-build-dirs.sh --delete   # remove orphans
```

## Versioning & releases

**Pre-1.0 semver.** While the version is `0.x`, a **minor** bump may carry breaking
API changes; a **patch** bump is bugfix-only and API-compatible. The crates.io crate
suite, the PyPI `purrdf` package, the npm `@blackcatinformatics/purrdf` package,
and the `CITATION.cff` citation record share **one** workspace version and ship in
lockstep — CI runs a version-coherence check that fails if the four sources
disagree.

**MSRV.** The supported floor is `rust-version` in `Cargo.toml` (currently **1.96**)
on the **stable** channel, and CI enforces it with a dedicated MSRV job. That floor
is what you build against as a consumer; it is unaffected by the toolchain
contributors run. Raising the MSRV is a notable, changelog-recorded change that,
pre-1.0, rides a minor bump.

**Development toolchain.** `rust-toolchain.toml` pins a **dated nightly** for local
work and CI gates, because nightly clippy and rustdoc carry lints stable lacks —
so a gate finding is a real finding, not a channel artifact. The *source* stays
nightly-free: there are no `#![feature(...)]` attributes anywhere in the workspace
and adding one is rejected, which is exactly what the MSRV job proves on every PR.
Release artifacts are built on stable. If you have `rustup` installed, the pin
applies automatically; `scripts/check-toolchain-pin.py` (part of `make check`)
fails if a CI workflow ever drifts from it.

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
