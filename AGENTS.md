<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# AI Developer Agent Guide (AGENTS.md)

Welcome, AI Agent! This file is your behavioral contract and instruction manual for
contributing to the PurRDF repository.

---

## 1. What this repository is

PurRDF is the **RDF 1.2 toolkit** that several downstream projects (notably
[`gmeow-ontology`](https://github.com/Blackcat-Informatics/gmeow-ontology)) use as
their **data-carrier backbone**. It must stay fast, deterministic, and boring:
one engine, one behavior, carried verbatim into Rust, Python, WebAssembly, and C.

Crate map (all under `crates/`, published names in `Cargo.toml`):

| Crate | Role |
|---|---|
| `purrdf` | Umbrella facade (RDF surface at root; `slice`/`shapes` as modules) |
| `purrdf-rdf` (`crates/rdf`) | Native text/XML/JSON-LD codecs, GTS adapters, describe, canonicalization |
| `purrdf-core` (`crates/rdf-core`) | Interned IR kernel, diagnostics, store traits, provenance, RDFC-1.0 |
| `purrdf-gts` (`crates/gts`) | GTS container engine (CBOR log, BLAKE3, COSE) |
| `purrdf-sparql-{algebra,eval,results}` | SPARQL 1.1/1.2 parser, evaluator, results |
| `purrdf-shapes` (`crates/shapes`) | SHACL Core validation |
| `purrdf-slice` (`crates/slice`) | Slice catalog, artifacts, ownership analysis |
| `purrdf-iri`, `purrdf-xsd`, `purrdf-events` | Zero-dependency foundations |
| `purrdf-wasm`, `purrdf-capi`, `bindings/python` | WASM, C-ABI, and PyO3 bindings |

## 2. Hard constraints (violating these fails CI or review)

* **NO Cargo features, ever.** The workspace has zero feature flags and
  `scripts/check-no-features.py` gates CI. PurRDF is a carrier; optionality
  changes semantics per consumer, which is forbidden. Do not add `[features]`,
  optional deps, or `cfg`-gated behavior differences.
* **Kernel ring-fence.** `purrdf-core` must never depend on oxigraph or PyO3.
  `purrdf-iri`, `purrdf-xsd`, and `purrdf-events` must keep **zero runtime
  dependencies**.
* **Everything is wasm-able.** Every release crate (all 13 published crates,
  `purrdf-wasm` included) must build for `wasm32-unknown-unknown` — CI
  hard-fails otherwise (`make wasm` locally). Never add a dependency that
  drags in threads, the filesystem, C toolchains, or wall-clock/RNG syscalls
  on the wasm path; crypto stays pure-Rust for exactly this reason.
* **Byte determinism.** Serializers and the GTS writer are byte-deterministic.
  If your change alters emitted bytes, you must update the affected goldens and
  say why in the PR. Never introduce iteration-order, time, or RNG dependence
  into output paths (hashers are fixed-key `ahash` for this reason).
* **Conformance corpora are the contract**: W3C SPARQL 1.1
  (`crates/sparql-conformance`), SHACL corpus, RDFC-1.0 fixtures
  (`crates/rdf/tests/fixtures/rdfc/`), and the **frozen** GTS vectors in
  `vectors/` (shared byte-exact with the other GTS engines — never regenerate or
  "fix" them here; the GTS wire format is governed in `gmeow-gts`).
* **Generated artifacts** under `generated/` are projections — never hand-edit;
  regenerate via `make metadata` (`scripts/check-generated.sh` gates drift).
* **Dependency versions live in one place**: `[workspace.dependencies]` in the
  root `Cargo.toml`. Member crates use `dep.workspace = true`. Do not pin a
  version inside a member manifest.
* **Lints are workspace-inherited** (`[workspace.lints]`, clippy pedantic +
  nursery). `cargo clippy --workspace --all-targets` must be warning-free.
  Prefer fixing code over `#[allow]`; a genuinely-right allow must be tightly
  scoped and carry a reason comment.
* **SPDX headers** on every source file: `MIT OR Apache-2.0` (docs may be
  `CC-BY-4.0`).

## 3. Commands

```bash
make check      # the full local gate: fmt, clippy, build, tests, hygiene
make test       # cargo test --workspace
make metadata   # regenerate + verify generated artifacts
make bench      # criterion benchmarks (report-only; not a gate)
```

Toolchain: `rust-toolchain.toml` pins nightly for development; CI builds and
tests on **stable**, which is the supported floor (`rust-version` in
`Cargo.toml`). Don't use nightly-only language features in library code.

## 4. Performance discipline

This library is a hot-path backbone. The IR is an immutable, value-interned
dataset (`TermId` = niche-optimized `NonZeroU32`, string arena, store-once
interner). When touching parse/serialize/eval paths:

* **Measure first** — layout and algorithm choices are justified by the
  criterion benches (`crates/rdf-core/benches/ir_layout.rs` et al.), not by
  assertion. Add or extend a bench when you claim a win.
* Avoid per-token/per-term `String` allocation; move values out of buffers
  instead of cloning; pre-size collections in parse loops.
* Hot maps use fixed-key `ahash` (see `crates/rdf-core/src/ir/builder.rs` for
  the canonical store-once interner pattern) — never default SipHash in a hot
  path, and never a randomly-seeded hasher in an output path.

## 5. Brand & naming

The project name is written **PurRDF** in prose (never PurrDF/PURRDF); all
package/crate/binary identifiers are lowercase `purrdf`. See
[`docs/BRAND.md`](./docs/BRAND.md). Logo/social assets follow the shared
black-cat family system — `#cat-head-core` is shared verbatim; only the
`#service-triple` group is purrdf-specific.

## 6. Releases

Tag-driven trusted publishing: `rust-v*` → crates.io (13 crates, ordered),
`py-v*` → PyPI (`purrdf`). See [`docs/RELEASE.md`](./docs/RELEASE.md). Version
is single-sourced in `[workspace.package]`. `purrdf-capi` and
`purrdf-sparql-conformance` are never published.

## 7. Provenance

This repo was extracted from `gmeow-ontology` and `gmeow-gts` — see
[`PROVENANCE.md`](./PROVENANCE.md) for source commits and
[`docs/CUTOVER.md`](./docs/CUTOVER.md) for the downstream cutover checklist.
ShEx support is planned but **not implemented** — do not claim it in docs.
