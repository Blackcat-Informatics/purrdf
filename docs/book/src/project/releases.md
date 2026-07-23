<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Versioning & Releases

PurRDF ships to three registries — the crates.io crate suite, the PyPI
`purrdf` package, and the npm `@blackcatinformatics/purrdf` package — from
**one** workspace version, in lockstep. The full process is
[`docs/RELEASE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/RELEASE.md).

## Pre-1.0 semver policy

While the version is `0.x`:

- a **minor** bump (`0.x` → `0.(x+1)`) may include breaking API changes;
- a **patch** bump (`0.x.y` → `0.x.(y+1)`) is bugfix-only and API-compatible.

All three published surfaces share one workspace version and are released
together. That coherence is enforced in CI: a version-coherence check fails
the build if the three version sources disagree.

## MSRV policy

The supported minimum Rust is `rust-version` in the root `Cargo.toml` —
currently **1.96** — pinned to the **stable** toolchain (the workspace is
nightly-free by policy) and enforced by a dedicated CI MSRV job. Raising the
MSRV is a notable change recorded in the changelog and, pre-1.0, rides a
minor bump.

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

Two crates are deliberately never published: `purrdf-capi` (built via
cargo-c, distributed as `libpurrdf`) and `purrdf-sparql-conformance` (the
test harness).

## Cutting a release

The coherent flow from `main` uses the `make` helpers so the three lanes can
never drift:

```sh
# 1. Bump all three version sources in lockstep (fails unless they end up equal).
make bump VERSION=0.2.2

# 2. Regenerate the changelog from the conventional-commit history.
make changelog

# 3. Review, then commit the release bump + changelog.
git add -A && git commit -m "chore(release): 0.2.2"

# 4. From an up-to-date main, run the full gate, then cut and push all three tags.
make release-tags VERSION=0.2.2
```

`make release-tags` refuses to run unless the working tree is clean, the
branch is `main` and synchronized with `origin/main`, the version check passes,
`VERSION` matches the tree, the release-notes section exists, and none of the
three tags already exists locally or remotely. It then runs the complete
`make check` gate itself, rechecks the clean synchronized state, and atomically
pushes the `rust-v`, `py-v`, and `npm-v` tags together. No tag is created before
the full gate passes. Each tag triggers its own lane, and the cargo lane
additionally publishes a GitHub Release built from the committed
`CHANGELOG.md`.

## Citing PurRDF

Releases carry a DOI; if you use PurRDF in research, please cite it — see
[`CITATION.cff`](https://github.com/Blackcat-Informatics/purrdf/blob/main/CITATION.cff)
in the repository.
