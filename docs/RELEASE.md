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

# 3. Regenerate the changelog from the conventional-commit history.
make changelog

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
- `purrdf-text`
- `purrdf-rdf`
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

crates.io currently requires the crate to exist before a Trusted Publisher can
be configured. Bootstrap publishes for new crate records therefore use an
explicit token. After those crate records exist, enable the Trusted Publisher
entries above and use the GitHub release workflow for future releases.

### Outstanding bootstrap: `purrdf-cdt`, `purrdf-text`, `purrdf-geo`

Three crates are in the release set above but **have no crates.io record**
(`https://crates.io/api/v1/crates/<name>` answers 404 for each while every
sibling answers 200). `purrdf-cdt` is the **fourth** crate in publish order,
`purrdf-text` the **thirteenth** and `purrdf-geo` the **seventeenth**; all three
are new crates whose records have never been created.

That list is not prose. It is `PURRDF_UNBOOTSTRAPPED_CRATES` in
[`scripts/release-crates.sh`](../scripts/release-crates.sh), a **ledger** the
preflight holds to the registry in **both** directions: a crate crates.io lacks
that the ledger does not name fails the preflight, and a ledger entry crates.io
now *has* a record for also fails it, so the ledger cannot go on naming a crate
someone has since bootstrapped. This section restates the ledger, and
`scripts/check-doc-claims.py` fails `make check` if it restates it wrongly.

**What that would cost without the preflight, and why the preflight exists.**
`cargo publish` cannot be undone, and this lane publishes one crate at a time in
dependency order — so a `rust-v*` tag would irreversibly publish the three crates
ahead of `purrdf-cdt` and only then fail. `purrdf-cdt` moved that failure point
EARLIER than any previous new crate did: it is a leaf over `purrdf-iri` +
`purrdf-xsd`, so it sorts near the front of the dependency order, and the damage
would stop after three crates rather than after six. **That is the counterfactual,
not current behaviour**: the preflight described below runs before any packaging
or publishing, so today a tag pushed in this state costs a red job and publishes
nothing at all.

This section previously also named `purrdf-datalog`, which has had a crates.io
record since 2026-07-31 and answers `0.12.0`. That was a stale entry, not a
missing record, and nothing could see it: the list lived only in this prose. It
is why the list is now a ledger with two gates on it. Offline,
`outstanding_bootstrap_claim` in
[`scripts/check-doc-claims.py`](../scripts/check-doc-claims.py) holds this
section — heading, crate names, publish-order ordinals and the anchor that links
here — to `PURRDF_UNBOOTSTRAPPED_CRATES` and `PURRDF_RELEASE_CRATES` on every
`make check`, so a crate that moves in the release set cannot leave a wrong
ordinal behind. Online, the preflight below holds the ledger itself to crates.io.
*Membership* is only ever decided by that second gate: whether a record exists is
a fact about the registry, not about this tree, and this section must never be
read in place of running the preflight.

The mechanism that makes it a refusal rather than a partial publish: the release job runs
[`scripts/check-crates-io-records.sh`](../scripts/check-crates-io-records.sh)
**before any packaging or publishing**; it queries `/api/v1/crates/<name>` for
every crate in the release set and fails the job naming the missing crate and
the bootstrap command below. Run it locally at any time:

```sh
bash scripts/check-crates-io-records.sh
```

Only a literal 404 counts as missing: crates.io answers 403 to a default
`User-Agent`, so the script sends the same `purrdf-release/<version>` agent the
publish loop uses, retries transient statuses, and treats any other answer as a
hard stop rather than as "missing".

**The record itself can only be created by a maintainer**, because creating it
is a token-authenticated, irreversible outward publish that a Trusted Publisher
is not permitted to perform. Do it once, from a clean local checkout, before the
next `rust-v*` tag:

```sh
CARGO_REGISTRY_TOKEN="${CARGO_TOKEN}" scripts/bootstrap-crates-io.sh
```

The version argument is optional and deliberately omitted here: with none given
the script reads the workspace version out of `cargo metadata`, so this command
cannot go stale the way a pinned literal in a document does.

The bootstrap script prints its full plan — which crates it will skip, publish,
and **create a record for** — before it runs any gate, so the irreversible part
is visible while it is still stoppable. After it completes, add a Trusted
Publisher entry for each newly created record using the table above; the
preflight then passes and the tag lane works unchanged.

`purrdf-python`, `purrdf-sparql-conformance`, `purrdf-cli`, and `purrdf-capi`
remain workspace crates, but they are not in this crates.io release lane.
`purrdf-python` is the PyPI extension package under `bindings/python`, the
conformance harness is an internal W3C fixture runner, `purrdf-cli` is the
native command-line package, and
the C ABI is a native artifact that should get a separate release lane if/when it
is shipped.

For the bootstrap publish from a clean local checkout:

```sh
CARGO_REGISTRY_TOKEN="${CARGO_TOKEN}" scripts/bootstrap-crates-io.sh
```

The script states its crates.io plan first (skip / publish / **create record**,
per crate), then runs the local release gates, refuses dirty source by default,
skips crate versions that already exist, and publishes crates in dependency
order.
It also verifies the published crate set with `cargo check --target
wasm32-unknown-unknown --lib`; if the target is not installed and `rustup` is
available, the script installs it before checking.

#### Verification is on, and why it was off

Each `cargo publish` — in this bootstrap script **and** in the tag-driven
[`release-cargo.yaml`](../.github/workflows/release-cargo.yaml) loop, which is
the same loop under an OIDC token — **verifies**: cargo unpacks the `.crate` it
is about to upload and builds it against the registry, so a wrong-version or
broken artifact is refused *before* the one step that cannot be undone. Both
loops used to pass `--no-verify`, and that flag was **load-bearing**, not
laziness: `purrdf-geo` dev-depends on `purrdf-rdf` and `purrdf-shapes`, and
while it was ordered before both, verification of crate 13 would have had to
resolve two sibling versions that did not exist on crates.io yet. Verification
resolves the packaged crate's *whole* graph, dev-dependencies included, even
though it builds only the lib. One forward dev-edge is enough to make
verification impossible for the entire set.

The fix was to move one crate, not to drop the flag: `purrdf-geo` now publishes
after `purrdf-shapes`, and
[`scripts/check-publish-order.py`](../scripts/check-publish-order.py) proves on
every `make check` — and again in the release workflow's verify step, at the
point of no return — that the release order is a topological order of normal
**and** dev-dependencies, that the release set is exactly the publishable
workspace members, and that the bootstrap ledger is in-set and in order. Its
`--self-test` perturbs each of those and requires the refusal.

Two `--no-verify` remain, both deliberate. The pre-loop `cargo package
--workspace` (in both lanes) packages every crate before *any* is published, so
verification there is impossible by construction; it exists to find a packaging failure
before the first irreversible upload rather than midway through the set. And
`PUBLISH_NO_VERIFY=true` restores the old loop behaviour for one run, for a
verification failure that is demonstrably not a broken artifact (a registry
outage mid-run) — with the understanding that your own build of the artifact
is then the last check before permanence.

`PUBLISH_COOLDOWN_SECONDS` defaults to `0`. crates.io's new-crate rate limit is
enforced at the publish itself: a limited `cargo publish` exits non-zero,
`set -e` stops the script before the next crate, nothing is half-uploaded, and
a re-run resumes because published versions are skipped. The old default of
`620` modelled the limit's ten-minute refill unconditionally and added about
half an hour of dead time to a three-record run. Set it only if a run actually
meets a 429 with records still to create.

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

The workflow first refuses outright if any crate in the release set has no
crates.io record (see [Outstanding
bootstrap](#outstanding-bootstrap-purrdf-cdt-purrdf-text-purrdf-geo)); that check runs before
packaging, so a missing record costs a red job rather than a half-published
release. It then publishes crates in dependency order and skips any
crate/version that already exists on crates.io, which keeps reruns safe after a
partial publish.

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
