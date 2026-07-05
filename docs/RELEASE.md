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

# 2. Regenerate the changelog from the conventional-commit history.
make changelog

# 3. Review, then commit the release bump + changelog.
git add -A && git commit -m "chore(release): 0.2.2"

# 4. Gate: fmt, clippy, tests, hygiene, and the version-coherence + wasm checks.
make check

# 5. From an up-to-date main, cut and push all three tags in one command.
make release-tags VERSION=0.2.2
```

`make release-tags` refuses to run unless the working tree is clean, the branch
is `main`, `scripts/check-versions.py` passes, and `VERSION` matches the tree —
then it creates and pushes `rust-v0.2.2`, `py-v0.2.2`, and `npm-v0.2.2`
together. Each tag triggers its own lane (below); the cargo lane additionally
publishes a GitHub Release built from the committed `CHANGELOG.md`.

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

Use that same publisher configuration for these crates:

- `purrdf-events`
- `purrdf-iri`
- `purrdf-xsd`
- `purrdf-gts`
- `purrdf-core`
- `purrdf-entail`
- `purrdf-sparql-algebra`
- `purrdf-sparql-results`
- `purrdf-sparql-eval`
- `purrdf-rdf`
- `purrdf-slice`
- `purrdf-shapes`
- `purrdf-shex`
- `purrdf-validate`
- `purrdf`
- `purrdf-wasm`

crates.io currently requires the crate to exist before a Trusted Publisher can
be configured. Bootstrap publishes for new crate records therefore use an
explicit token. After those crate records exist, enable the Trusted Publisher
entries above and use the GitHub release workflow for future releases.

`purrdf-python`, `purrdf-sparql-conformance`, `purrdf-entail`, and `purrdf-capi`
remain workspace crates, but they are not in this crates.io release lane.
`purrdf-python` is the PyPI extension package under `bindings/python`, the
conformance harness is an internal W3C fixture runner, `purrdf-entail` is an
internal PurRDF entailment/reasoning crate with no publishable dependents, and
the C ABI is a native artifact that should get a separate release lane if/when it
is shipped.

For the bootstrap publish from a clean local checkout:

```sh
CARGO_REGISTRY_TOKEN="${CARGO_TOKEN}" scripts/bootstrap-crates-io.sh 0.1.1
```

The script runs the local release gates, refuses dirty source by default, skips
crate versions that already exist, and publishes crates in dependency order.
It also verifies the published crate set with `cargo check --target
wasm32-unknown-unknown --lib`; if the target is not installed and `rustup` is
available, the script installs it before checking.

## Changelog and release notes

The changelog is generated deterministically from the conventional-commit
history by [git-cliff](https://git-cliff.org/), configured in `cliff.toml`.
Install the pinned version once:

```sh
cargo install git-cliff --version 2.13.1 --locked --no-default-features
```

Regenerate `CHANGELOG.md` as part of the release commit. Run `make bump` **first**:
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
never drift, and the workflow makes no repository commits. Always run
`make changelog` and commit the result **before** pushing the release tag.

## Tag Release

After the release commit is on `main` and all Trusted Publisher entries exist,
push one release tag:

```sh
git tag rust-v0.1.5
git push origin rust-v0.1.5
```

The workflow publishes crates in dependency order and skips any crate/version
that already exists on crates.io, which keeps reruns safe after a partial
publish.

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
(`make wasm-pkg`), runs the Node real-execution suite, packs the tarball,
attests provenance + SPDX SBOM, and publishes with `--access public`
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
