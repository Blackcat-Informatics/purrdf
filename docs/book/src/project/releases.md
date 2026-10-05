<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Versioning & Releases

PurRDF ships to three registries — the crates.io crate suite, the PyPI
`purrdf` and `purrdf-rdflib` distributions, and the npm
`@blackcatinformatics/purrdf` package — from
**one** workspace version, in lockstep. The full process is
[`docs/RELEASE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/RELEASE.md).

## Semver policy from 1.0.0

From 1.0.0 the suite follows semantic versioning in full:

- a **breaking** change bumps the **major** version. A commit carrying `!` or
  a `BREAKING CHANGE:` footer is a major-bump trigger, and the changelog marks
  each such entry **BREAKING**;
- a **minor** bump is additive and API-compatible;
- a **patch** bump is bugfix-only.

That is what the version number commits to; it is a policy statement, not a
claim of stability beyond what semver means. All three published surfaces
share one workspace version and are released together, and a
version-coherence check in CI fails the build if the version sources
(`Cargo.toml`, `pyproject.toml`, `package.json`, `CITATION.cff`) disagree.

The one exception is the C ABI. `libpurrdf`'s
[`purrdf.h`](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/rdf-capi/include/purrdf.h)
carries its own `PURRDF_ABI_MAJOR.PURRDF_ABI_MINOR` (currently **0.9**), bumped
on every exported-signature change, pinned by
`crates/rdf-capi/tests/abi_signatures.rs`, and read back at runtime through
`purrdf_abi_version`. It is versioned separately from the workspace and stays
`0.x`: it is not frozen, and the workspace's 1.0.0 makes no promise about it.

## MSRV policy

The supported minimum Rust is `rust-version` in the root `Cargo.toml` —
currently **1.98** — on the **stable** channel, enforced by a dedicated CI
MSRV job that sets `RUSTUP_TOOLCHAIN` explicitly and asserts the compiler it
measured really is 1.98. Raising the MSRV is a notable change recorded in the
changelog; it rides a **minor** bump and never ships in a patch release.

The MSRV is a promise to consumers; the development toolchain is a tool
choice, and the two are orthogonal. `rust-toolchain.toml` names a **floating
nightly** for local work and the CI gates, because nightly clippy and rustdoc
carry lints stable lacks and its default borrow checker is the stronger one —
but by policy the source uses zero nightly (unstable) features (no
`#![feature(...)]` attributes, which the MSRV job proves on every change), and the release lanes build every
published artifact on stable.

## Tag-driven trusted publishing

Releases are tag-driven: `rust-v<version>` publishes the crate suite to
crates.io, `py-v<version>` publishes to PyPI, and `npm-v<version>` publishes
the wasm package to npm. The lanes share the supply-chain posture of the
cargo lane:

- publication uses **Trusted Publishing** through GitHub Actions OIDC — no
  long-lived registry secret;
- the privileged publish jobs use pinned actions and no dependency cache;
- every `.crate` package receives a GitHub **build-provenance attestation**;
- the package set receives an **SPDX SBOM** and SBOM attestation;
- the release crate set is checked on `wasm32-unknown-unknown` before
  publishing;
- every workspace crate version must match the tag version.

Every functional version of every crate in the 31-crate release set is
published by that lane. Each existing crate record is locked on crates.io with
*Require trusted publishing* (`trustpub_only`), so an API token cannot publish
a new version of any of them: crates.io answers with
`403 Forbidden: New versions of this crate can only be published using Trusted Publishing`.

Set up new crate records **before tagging**. A token creates each missing
record by publishing an isolated, empty, dependency-free **0.0.0** package;
the real workspace crate keeps its functional release version. Configure its
Trusted Publisher entry and enable *Require trusted publishing*, then reconcile
`PURRDF_UNBOOTSTRAPPED_CRATES` in `scripts/release-crates.sh`. All 31 records
must exist, all publisher entries must be configured, every record must be
locked, and the ledger must be empty before the functional release begins.
`scripts/check-crates-io-records.sh --require-all` verifies the public records
and locks; publisher entries require separate configuration confirmation.

The tagged trusted-publishing run then publishes the functional versions in
dependency order. After those versions are verified, yank the empty **0.0.0**
versions using a token with the separate yank permission, retaining the records
and publisher settings. The complete setup and historical bootstrap receipts
are in the release process document linked above.

Eleven workspace members are deliberately never published to crates.io:
`purrdf-capi` (built via cargo-c, distributed as `libpurrdf`),
`purrdf-sparql-conformance` (the test harness), `purrdf-hash-conformance` (the
frozen-vector suites of `purrdf-hash`), `purrdf-cli` (the `purrdf`
binary), `purrdf-envelope-probe` (the micro-hardware envelope capture tool),
`purrdf-bench` (benchmark tooling), `purrdf-alloc-probe` (the shared counting
allocator the allocation tests and benches measure with), `purrdf-testkit` (the
shared test support: goldens, temporary paths, frozen vectors and the
`harness = false` runner), `wasm-link` (the wasm package's post-link step),
`helper-census` (the structural census behind the shared-helpers gate), and
`purrdf-python` (the extension crate, which ships to PyPI via maturin instead).

`purrdf-alloc-probe` and `purrdf-testkit` are the only two of the eleven that
published crates depend on, and they reach them solely through
`[dev-dependencies]`. Their root `[workspace.dependencies]` entries are
therefore path-only, with no `version` key, which is what makes cargo drop them
from the packaged manifest: an unpublished crate cannot be a versioned
dependency of a published one, and `cargo publish`'s verification step resolves
dev-dependencies too.

## Cutting a release

The coherent flow from `main` uses the `make` helpers so the three lanes can
never drift:

```sh
# 1. Bump all three version sources in lockstep (fails unless they end up equal).
make bump VERSION=0.2.2

# 2. Regenerate the committed C-ABI header from the bumped crate version.
make capi-header

# 3. Complete the changelog and reviewed summary, preserving migration guidance.
# Rename the Unreleased section to the bumped version and release date.
# Use make changelog only for history-generated notes.
# Add the reviewed short summary at docs/releases/<version>.md.

# 4. Review, then commit the bump, generated header, changelog and summary.
git add -A && git commit -m "chore(release): 0.2.2"

# 5. From an up-to-date main, run every release gate, then push all three tags.
make release-tags VERSION=0.2.2
```

`make release-tags` refuses to run unless the working tree is clean, the
branch is `main` and synchronized with `origin/main`, the version and crates.io
record/lock checks pass, `VERSION` matches the tree, the full changelog section
and validated reviewed summary exist, and none of the three tags already exists
locally or remotely. It then
runs these gates in order:

1. `make check`: the Rust and wasm workspace gate and repository hygiene.
2. `make capi-check`: the generated C-ABI/header check and linked C smoke.
3. `make pytest`: the native Python binding suite.
4. `make python-release-check`: build and audit both Python wheel/sdist pairs,
   check them with the pinned publisher tools, and install them together.
5. `make wasm-pkg-test`: the optimized size-gated npm/wasm package tests.
6. `make capi-bundle`: build and audit the native C distribution, including
   recipient notices and relocated linked smoke.

Only after every surface passes does it recheck the clean synchronized state
and atomically push the `rust-v`, `py-v`, and `npm-v` tags together. No tag is
created before the complete cross-surface preflight passes. Each tag triggers
its own lane, and the cargo lane additionally publishes a GitHub Release using
the committed reviewed summary in `docs/releases/<version>.md`, linked to the
complete changelog. The shared checker validates the exact version and 64 KiB
UTF-8 size ceiling before tags and publication; no notes are truncated.

## Citing PurRDF

Releases carry a DOI; if you use PurRDF in research, please cite it — see
[`CITATION.cff`](https://github.com/Blackcat-Informatics/purrdf/blob/main/CITATION.cff)
in the repository.
