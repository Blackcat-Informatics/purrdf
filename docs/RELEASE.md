<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Release Process

PurRDF publishes Rust crates to crates.io from the GitHub Actions workflow
`.github/workflows/release-cargo.yaml`.

The release lane follows the `gmeow-gts` cargo release pattern:

- release tags are `rust-v<version>`;
- every workspace crate version must match the tag version;
- the workflow uses pinned actions and no dependency cache in the privileged
  publish job;
- crates are packaged before publication;
- the release crate set is checked on `wasm32-unknown-unknown`;
- each `.crate` package receives a GitHub build-provenance attestation;
- the package set receives an SPDX SBOM and SBOM attestation;
- crates.io publication uses Trusted Publishing through GitHub Actions OIDC,
  not a long-lived repository secret.

## Cutting a release

The suite ships **one** version to crates.io, PyPI, and npm. Cutting a release
is a single coherent flow from `main`, using the version-coherence gate and the
`make` helpers so the three lanes can never drift:

```sh
# 1. Bump all three version sources in lockstep (fails unless they end up equal).
make bump VERSION=0.2.2

# 2. Regenerate the committed C-ABI header from the bumped crate version.
make capi-header

# 3. Complete the release notes, preserving existing migration guidance.
# Rename the Unreleased section to the bumped version and release date.
# Use make changelog only for history-generated notes (see below).

# 4. Review, then commit the release bump, generated header, and changelog.
git add -A && git commit -m "chore(release): 0.2.2"

# 5. From an up-to-date main, run every release gate, then push all three tags.
make release-tags VERSION=0.2.2
```

`make release-tags` refuses to run unless the working tree is clean, the branch
is `main` and synchronized with `origin/main`, `scripts/check-versions.py`
passes, `VERSION` matches the tree, the release-notes section exists, and none
of the three tags already exists locally or remotely. It then runs the Rust and
wasm workspace gate, the generated C-ABI/header check, the native Python binding
suite, and the optimized size-gated npm/wasm package tests. Only after every
surface passes does it recheck the clean synchronized state and atomically push
`rust-v0.2.2`, `py-v0.2.2`, and `npm-v0.2.2` together. No tag is created before
the complete cross-surface preflight passes. Each tag triggers its own lane
(below); the cargo lane additionally publishes a GitHub Release built from the
committed `CHANGELOG.md`.

The per-lane tag commands in the sections below remain valid for a single-lane
re-release, but the coherent path above is the default.

## Trusted Publisher Setup

Configure one crates.io Trusted Publisher entry per crate:

| Field | Value |
| --- | --- |
| Publisher | GitHub Actions |
| Owner | `Blackcat-Informatics` |
| Repository | `purrdf` |
| Workflow | `release-cargo.yaml` |
| Environment | `(none)` |

Use that same publisher configuration for these crates — the list below is the
one in [`scripts/release-crates.sh`](../scripts/release-crates.sh), which the
workflow, the bootstrap script and the crates.io preflight all source, and which
`scripts/check-doc-claims.py` checks this list against:

- `purrdf-events`
- `purrdf-iri`
- `purrdf-xsd`
- `purrdf-cdt`
- `purrdf-gts`
- `purrdf-core`
- `purrdf-columnar`
- `purrdf-datalog`
- `purrdf-entail`
- `purrdf-sparql-algebra`
- `purrdf-sparql-results`
- `purrdf-sparql-eval`
- `purrdf-hnsw`
- `purrdf-text`
- `purrdf-rdf`
- `purrdf-markdown`
- `purrdf-json`
- `purrdf-slice`
- `purrdf-shapes`
- `purrdf-geo`
- `purrdf-shex`
- `purrdf-validate`
- `purrdf`
- `purrdf-wasm`

The *order* of that list is gated too: `scripts/check-publish-order.py` proves
on every `make check` that it is a topological order of normal **and**
dev-dependencies, which is what lets `cargo publish` verify every crate — see
[Verification](#verification-is-on-and-why-it-was-off).

crates.io requires a crate to exist before a Trusted Publisher can be
configured for it, and refuses to create one from a Trusted Publishing token —
its publish handler answers `Trusted Publishing tokens do not support creating
new crates. Publish the crate manually, first`. Creating a record is therefore
the **only** thing an API token does in this release process; later versions
are published through Trusted Publishing.

### New crates: set up publishing before tagging

Create new crate records before cutting the release tags. This lets the
maintainer configure Trusted Publishing while the release checks run, so the
first tagged run can publish the complete workspace in dependency order.

1. Complete the real crate in the workspace at the intended release version.
   Separately, create an isolated package outside the workspace with the same
   crate name, version `0.0.0`, no dependencies, and an empty `#![no_std]`
   library. Include the normal license and repository metadata, and a README
   stating that this version creates the registry record and exposes no runtime
   API. Keep the workspace version and dependency requirements at the real
   release version; the isolated package is not part of the release source.
2. With `bootstrap_dir` pointing to that isolated package, verify and publish
   it using a token authorized to create the crate. Verification stays enabled:

   ```sh
   rustup run stable cargo publish --dry-run --manifest-path "$bootstrap_dir/Cargo.toml"
   CARGO_REGISTRY_TOKEN="${CARGO_TOKEN:?CARGO_TOKEN is required}" \
     rustup run stable cargo publish --manifest-path "$bootstrap_dir/Cargo.toml"
   ```

3. On the new crate's crates.io **Settings** page, add the Trusted Publisher
   using the table above and enable **Require trusted publishing**. Repeat for
   every new crate. The publisher entry and the lock are separate requirements;
   the public record's `trustpub_only` field proves the lock, not the entry.
4. Once each record exists, remove it from `PURRDF_UNBOOTSTRAPPED_CRATES` in
   `scripts/release-crates.sh` and update the bootstrap status below. Commit
   those changes with the release preparation. Before tagging, require both
   checks to pass:

   ```sh
   python3 scripts/check-doc-claims.py
   bash scripts/check-crates-io-records.sh
   ```

5. Follow the normal release steps above. With the ledger empty and every
   publisher configured, the trusted lane publishes every functional crate
   version in one run, without pausing for token publication. The isolated
   `0.0.0` package needs no unreleased dependencies; the workspace bootstrap
   script instead publishes real implementations after their dependencies are
   available on the registry.
6. After the functional release is published, set `new_crate` to its registered
   name and yank its `0.0.0` version using a token with the separate **yank**
   permission:

   ```sh
   CARGO_REGISTRY_TOKEN="${CARGO_TOKEN:?a token with yank permission is required}" \
     rustup run stable cargo yank --version 0.0.0 "$new_crate"
   ```

   Yanking prevents new dependency resolutions from choosing the bootstrap
   version while retaining the crate record and publisher settings. Keep those
   records; deleting a crate would undo the setup. Yank can be reversed with
   `cargo yank --undo --version 0.0.0 "$new_crate"`.

### Outstanding bootstrap: `purrdf-hnsw`

One crate is in the release set above with no crates.io record yet:
`purrdf-hnsw` is the **thirteenth** in publish order. It follows
`purrdf-sparql-eval`, `purrdf-core` and `purrdf-xsd`, its workspace
dependencies, and precedes `purrdf-text`. `PURRDF_UNBOOTSTRAPPED_CRATES` in
[`scripts/release-crates.sh`](../scripts/release-crates.sh) names it; the
ledger is held to the registry in both directions by the preflight. The entry
leaves once its record exists. No crate in the release set depends on
`purrdf-hnsw`, so the lane publishes the twelve crates ahead of it, skips it
visibly, and continues through every later crate; only `purrdf-hnsw` itself
waits for the token step described in
[New crates: set up publishing before tagging](#new-crates-set-up-publishing-before-tagging).

`purrdf-markdown` and `purrdf-json` have **0.0.0** bootstrap records, created by
token publication solely to configure Trusted Publishing. Those versions
expose no runtime API; their functional release is **2.0.0**. With the records
and publisher settings established, the trusted release lane publishes all 24
crates in dependency order. The 0.0.0
versions can be yanked after the functional release, retaining the crate
records and publisher settings.

The three crates that once had no crates.io record — `purrdf-cdt`,
`purrdf-text`, `purrdf-geo` — were bootstrapped during the 0.13.0 release:
`purrdf-cdt` by a token publish at 0.13.0 once its dependencies were up, and
`purrdf-text`/`purrdf-geo` as `0.0.0-bootstrap` placeholder records (their
first real version is 1.0.0, published through Trusted Publishing like every
sibling). All three carry a Trusted Publisher entry and the
*Require trusted publishing* lock.

#### The `0.0.0-bootstrap` placeholders — yanked

`purrdf-text` and `purrdf-geo` each carried a `0.0.0-bootstrap` version: an
empty lib published under deadline purely to create the crate record so Trusted
Publishing could be enabled on it. Nothing ever resolved to them — `1.0.0` is
`max_version` for both — but a version list is read by people, and a `0.0.0`
release of a crate that never had one is a puzzle for every future reader.

Both are now **yanked**, which hides them from resolution without deleting them:

```sh
cargo yank --version 0.0.0-bootstrap purrdf-text
cargo yank --version 0.0.0-bootstrap purrdf-geo
```

Two things to carry forward if a future release ever needs this pattern again:

* **Yank is a separate permission from publish.** Every PurRDF crate is locked
  to Trusted Publishing for publishing, but that lock does not grant or deny
  yank — that needs an API token carrying the yank scope, and the release
  workflow's trusted session cannot do it. This step is always manual.
* **It is reversible** (`cargo yank --undo --version …`), which is what makes it
  the right tool here rather than a request to crates.io support.

For new crate records, use the upfront setup procedure above. It keeps
registry setup separate from the functional release and lets the first trusted
release run complete without a bootstrap interleave.


## Changelog and release notes

The changelog includes reviewed migration guidance and entries generated from
conventional-commit history by [git-cliff](https://git-cliff.org/), configured
in `cliff.toml`. Preserve existing hand-authored notes when preparing a release:
complete the entries and rename `## [Unreleased]` to the bumped version and
release date. Commit subjects do not capture all consumer migration steps.

`make changelog` regenerates the whole file and refuses to overwrite
hand-authored release notes. For history-generated notes, install the pinned
generator once:

```sh
cargo install git-cliff --version 2.13.1 --locked --no-default-features
```

When generating from history, run `make bump` **first**:
`make changelog` reads the just-bumped workspace version out of `Cargo.toml` and
passes it to git-cliff as `--tag rust-v<version>`, so the pending (still untagged)
commits are stamped under a real `## [<version>]` header instead of landing in
`## [Unreleased]`. That is the header the release workflow later slices out of the
committed `CHANGELOG.md` verbatim, so the version being cut must already be the tree
version when you regenerate:

```sh
make changelog   # stamps the bumped version as the changelog release header,
                 # then re-checks that no #NNN tokens leaked
```

`cliff.toml` groups entries by conventional-commit type, treats the `rust-v*`
tags as the release boundaries, and strips every `#NNN` issue/PR token so the
committed changelog stays clean under the repository's issue-reference lint.
The generation is offline and order-stable: running `make changelog` twice on
the same history (at the same tree version) yields byte-identical output.

The GitHub Release notes are **not** regenerated at tag time. The
`release-cargo.yaml` workflow slices the section for the tagged version straight
out of the committed `CHANGELOG.md` and attaches it to a GitHub Release named
for the `rust-v*` tag — so the release notes and the committed changelog can
never drift, and the workflow makes no repository commits. Complete and commit
the release's `CHANGELOG.md` section **before** pushing the release tag.

## Tag Release

After the release commit is on `main` and all Trusted Publisher entries exist,
push one release tag:

```sh
git tag rust-v0.1.5
git push origin rust-v0.1.5
```

The workflow first refuses outright if any crate in the release set has no
crates.io record and is not in the bootstrap ledger, or has a record that is
not locked to Trusted Publishing (see [bootstrap status](#outstanding-bootstrap-purrdf-hnsw)).
Every release crate the ledger does not name must have its record and lock
before packaging. The lane publishes crates in dependency order and skips any
crate/version already present on crates.io. A partially completed release
resumes with `gh run rerun <run-id>`.

## PyPI Release

The Python package is published by `.github/workflows/release-pypi.yaml` from
tags named `py-v<version>`. The workflow builds `bindings/python`, verifies that
the tag matches both `bindings/python/pyproject.toml` and
`bindings/python/Cargo.toml`, attests the Python distributions, attaches an SPDX
SBOM, and publishes to PyPI through Trusted Publishing.

Configure the PyPI pending publisher exactly as:

| Field | Value |
| --- | --- |
| Project | `purrdf` |
| Publisher | GitHub |
| Repository | `Blackcat-Informatics/purrdf` |
| Workflow | `release-pypi.yaml` |
| Environment | `(none)` |

The Python extension wheel uses the workspace Rust `release` profile. That
profile enables portable high-optimization settings: `opt-level = 3`, fat LTO,
one codegen unit, and stripped symbols. It deliberately does not use
`target-cpu=native`, because PyPI wheels must stay portable beyond the GitHub
runner CPU.

After the release commit is on `main` and the pending publisher is configured:

```sh
git tag py-v0.1.5
git push origin py-v0.1.5
```

## npm Release

`release-npm.yaml` publishes the `@blackcatinformatics/purrdf` ESM/wasm
package (`crates/rdf-wasm/js/`) on `npm-v*` tags. The **first** publish is
bootstrapped by the `NPM_TOKEN` repository secret (a trusted publisher can
only be configured once the package exists); after that, configure the
trusted publisher on npmjs.com and delete the token + secret — the workflow
switches to **npm trusted publishing** (OIDC) automatically:

| Field | Value |
| --- | --- |
| Publisher | GitHub Actions |
| Organization or user | `Blackcat-Informatics` |
| Repository | `purrdf` |
| Workflow filename | `release-npm.yaml` |
| Environment | `(none)` |

The workflow verifies the tag against `crates/rdf-wasm/js/package.json`,
builds the wasm artifact with the pinned `wasm-bindgen-cli` and `wasm-opt`
(`make wasm-pkg`), installs the pinned npm dev tools with `npm ci`, runs
the npm package gate (`npm run check`:
TypeScript, Node, and packed-tarball smoke), packs the
tarball, attests provenance + SPDX SBOM, and publishes with `--access public`
(npm's own sigstore provenance is added automatically).

The js package version is bumped by hand in `crates/rdf-wasm/js/package.json`
(it is not read from the workspace):

```sh
git tag npm-v0.1.5
git push origin npm-v0.1.5
```

## Verification

Download a published crate and verify its GitHub attestation:

```sh
VERSION=0.1.5
CRATE=purrdf
curl -L "https://crates.io/api/v1/crates/${CRATE}/${VERSION}/download" \
  -o "${CRATE}-${VERSION}.crate"
gh attestation verify "${CRATE}-${VERSION}.crate" \
  --repo Blackcat-Informatics/purrdf
```

Verify the SBOM predicate type for an attested crate:

```sh
gh attestation verify "${CRATE}-${VERSION}.crate" \
  --repo Blackcat-Informatics/purrdf \
  --predicate-type https://spdx.dev/Document/v2.3
```
