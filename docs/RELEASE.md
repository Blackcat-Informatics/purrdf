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
- `purrdf-sparql-algebra`
- `purrdf-sparql-results`
- `purrdf-sparql-eval`
- `purrdf-rdf`
- `purrdf-slice`
- `purrdf-shapes`
- `purrdf-shex`
- `purrdf`
- `purrdf-wasm`

crates.io currently requires the crate to exist before a Trusted Publisher can
be configured. Bootstrap publishes for new crate records therefore use an
explicit token. After those crate records exist, enable the Trusted Publisher
entries above and use the GitHub release workflow for future releases.

`purrdf-python`, `purrdf-sparql-conformance`, and `purrdf-capi` remain workspace
crates, but they are not in this crates.io release lane. `purrdf-python` is the
PyPI extension package under `bindings/python`, the conformance harness is an
internal W3C fixture runner, and the C ABI is a native artifact that should get a
separate release lane if/when it is shipped.

For the bootstrap publish from a clean local checkout:

```sh
CARGO_REGISTRY_TOKEN="${CARGO_TOKEN}" scripts/bootstrap-crates-io.sh 0.1.1
```

The script runs the local release gates, refuses dirty source by default, skips
crate versions that already exist, and publishes crates in dependency order.
It also verifies the published crate set with `cargo check --target
wasm32-unknown-unknown --lib`; if the target is not installed and `rustup` is
available, the script installs it before checking.

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
