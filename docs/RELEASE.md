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

crates.io requires a crate to exist before a Trusted Publisher can be
configured for it, and refuses to create one from a Trusted Publishing token —
its publish handler answers `Trusted Publishing tokens do not support creating
new crates. Publish the crate manually, first`. Creating a record is therefore
the **only** thing an API token does in this release process, and the next
section is exact about how little that is.

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

#### What a token can do, and what it cannot

**The token lane creates records. It publishes nothing else.** Every existing
PurRDF crate on crates.io is locked with crates.io's per-crate *Require trusted
publishing* setting — `"trustpub_only": true` in the public record JSON of all
eighteen — and crates.io answers a token publish of a locked crate with a 403.
That was established by an actual publish attempt, not by reading about it:
`scripts/bootstrap-crates-io.sh 0.13.0`, run with a valid `CARGO_TOKEN` from a
clean tree, packaged all 21 crates, verified `purrdf-events`, and on its
**first** upload received

```text
error: failed to publish purrdf-events v0.13.0 to registry at https://crates.io

Caused by:
  the remote server responded with an error (status 403 Forbidden): New versions of this crate can only be published using Trusted Publishing (see https://crates.io/docs/trusted-publishing).
```

Nothing landed (`purrdf-events/0.13.0` answered 404 afterwards). Until then
this document and that script both assumed the token lane could publish the
whole 21-crate set; that has been false since the records were locked. The
real division of labour:

| Lane | Publishes |
| --- | --- |
| API token (`scripts/bootstrap-crates-io.sh`) | the first-ever version of a crate with **no record** — nothing else |
| Trusted Publishing (`rust-v*` tag, `release-cargo.yaml`) | **every** subsequent version of **every** crate, all 21, the three new ones included |

The bootstrap script therefore walks `PURRDF_UNBOOTSTRAPPED_CRATES` and only
that: a ledger crate that already has a record is refused by name (a token
walking into eighteen 403s in a row fails safe, but it is wrong, not strict),
and `scripts/check-crates-io-records.sh` refuses any record that is not locked,
naming the setting to enable. Both refusals, and their valid neighbours, are
exercised offline by each script's `--self-test` on every `make check`.

#### The deadlock

Creating a record is a real `cargo publish`, and `cargo publish` — with **or
without** `--no-verify` — resolves the packaged crate's dependency graph against
the registry to write its lockfile. The three ledger crates depend on
`purrdf-iri`, `purrdf-xsd`, `purrdf-core` and `purrdf-sparql-eval`, and as
dev-dependencies on `purrdf-sparql-results`, `purrdf-rdf`, `purrdf-shapes` and
`purrdf-sparql-algebra`, all at the workspace version. At 0.13.0, verbatim, on
the stable cargo the lane uses and on the pinned nightly alike:

```text
$ cargo publish -p purrdf-cdt --no-verify --dry-run --locked
error: failed to prepare local package for uploading

Caused by:
  failed to select a version for the requirement `purrdf-iri = "^0.13.0"`
  candidate versions found which didn't match: 0.12.0, 0.11.0, 0.10.0, ...
  location searched: crates.io index
  required by package `purrdf-cdt v0.13.0 (…/crates/cdt)`
```

And the trusted lane cannot simply go first: `purrdf-cdt` is a *normal*
dependency of `purrdf-core`, `purrdf-sparql-algebra` and `purrdf-sparql-eval`,
and `purrdf` depends on `purrdf-text` and `purrdf-geo`, so `purrdf-core 0.13.0`
cannot be published before `purrdf-cdt 0.13.0` exists any more than
`purrdf-cdt` can before `purrdf-iri`. The two lanes block each other in both
directions, and the release is published by **interleaving** them.

#### The procedure: the interleave

The trusted lane publishes everything it can, skips each ledger crate, and
**stops cleanly at the first crate that depends on a skipped one**; the token
creates that ledger crate's record — its dependencies now exist, so the publish
is *verified* — the maintainer enables Trusted Publishing on the new record, and
the same workflow run is re-run and resumes where it stopped. The loop is
[`scripts/publish-release-crates.sh`](../scripts/publish-release-crates.sh); its
stop condition is computed from `cargo metadata` over every dependency kind
(verification resolves dev-dependencies too, and a dev-edge onto a *skipped*
crate fails exactly like a normal one), never hand-listed, and its `--self-test`
runs the whole interleave against a mock registry on every `make check`,
checking each stop against an independently computed first dependent. For the
current graph that is **three tag runs and two token steps**, because
`purrdf-text` and `purrdf-geo` share their first dependent (`purrdf`) and are
created in one token pass.

The workflow is tag-triggered only, so a stopped run is resumed with
`gh run rerun <run-id>` — same tag, same SHA, same workflow file — never by
re-pushing the tag. The GitHub Release is created only by the run that reports
the set complete. The sequence, with the exact commands and the messages to
expect (`gh run list --workflow release-cargo.yaml` gives the run id):

```sh
# 1. Tag. The lane publishes purrdf-events, purrdf-iri, purrdf-xsd, purrdf-gts;
#    skips purrdf-cdt ("skipping purrdf-cdt: bootstrap pending"); stops with
#      STOP: purrdf-core 0.13.0 depends on purrdf-cdt 0.13.0 (normal), which is in
#      PURRDF_UNBOOTSTRAPPED_CRATES and not on crates.io yet.
#    The job is green; the "Release incomplete" step names what comes next.
make release-tags VERSION=0.13.0

# 2. Token step 1, from a clean checkout of rust-v0.13.0. The plan says
#      CREATE RECORD  purrdf-cdt (no crates.io record; dependencies on crates.io: …)
#      DEFER          purrdf-text (… its dependencies are not on crates.io yet …)
#      DEFER          purrdf-geo  (…)
#    and it creates purrdf-cdt, verified.
git checkout rust-v0.13.0
scripts/bootstrap-crates-io.sh --plan
CARGO_REGISTRY_TOKEN="${CARGO_TOKEN}" scripts/bootstrap-crates-io.sh

# 3. On crates.io, for purrdf-cdt: add the Trusted Publisher entry (table above)
#    AND enable "Require trusted publishing". This release never touches
#    purrdf-cdt again (it is published), so a missed entry bites at the NEXT
#    release — a 403 at purrdf-cdt after the crates ahead of it — while a missed
#    lock makes step 4 refuse in the preflight, by design; and the lock can only
#    be enabled once the entry exists. Then resume the run:
gh run rerun <run-id>

# 4. The lane resumes at purrdf-core, publishes through purrdf-validate, skips
#    purrdf-text and purrdf-geo, and stops with
#      STOP: purrdf 0.13.0 depends on purrdf-text 0.13.0 (normal),purrdf-geo 0.13.0 (normal), …

# 5. Token step 2 (the plan shows purrdf-cdt as "skip … created by an earlier
#    run" and CREATE RECORD for purrdf-text and purrdf-geo):
CARGO_REGISTRY_TOKEN="${CARGO_TOKEN}" scripts/bootstrap-crates-io.sh

# 6. On crates.io, for purrdf-text AND purrdf-geo: Trusted Publisher entry AND
#    "Require trusted publishing". Then resume once more:
gh run rerun <run-id>
#    The lane publishes purrdf and purrdf-wasm, reports
#      COMPLETE: all 21 release crates are on crates.io at 0.13.0
#    and creates the GitHub Release.
```

After the release: remove the three crates from `PURRDF_UNBOOTSTRAPPED_CRATES`
and from this section (the offline gate holds the two together), on `main`. Not
before — the tag's tree is frozen while its release is in flight, which is why
the preflight tolerates a ledgered crate whose record exists *at the version
being released* (it reports it as `created … by this release's token step`)
and still refuses one whose record exists at any other version as stale.

The bootstrap script is the token step, and only that: it walks the ledger,
creates in one pass every ledger crate whose dependencies are on crates.io at
the target version, **defers** the rest by name — with the trusted-lane run
that will publish their dependencies as the next step — and refuses outright
when it can create nothing (`REFUSING: nothing can be created yet`), when a
ledger crate already has a record at another version, or when the ledger is
empty. `--plan` runs only that preflight and needs no token. It refuses a dirty
tree by default and skips a ledger crate whose version an earlier pass created.

#### The shapes that were rejected, and why

**`--no-verify` at the release version, token first.** Proposed on the
assumption that skipping verification skips dependency resolution. It does
not: the error above is `cargo publish --no-verify --dry-run`, and it is
cargo's lockfile resolve, which runs before any upload with or without the
flag. There is nothing for `--no-verify` to buy at the point the token step
becomes possible either — the dependencies exist, so the publish verifies.

**A temporary unlock.** Disable *Require trusted publishing* on the eighteen
existing crates, token-publish all 21 in order with the old whole-set loop,
re-lock. One run, no workflow change — and 0.13.0 of every crate becomes a
token publish with no Trusted Publishing provenance, no build-provenance
attestation and no SBOM attestation (both are made by the workflow), plus
thirty-six hand flips of a setting whose every change emails every owner.
That discards the posture the lane exists for.

**A placeholder record.** Token-publish a dependency-free stub (an empty
`lib.rs` at `0.0.0`) under each name from a throwaway crate, lock it, yank it
once the real version is up. One tag run, fully verified, no interleave — but a
yanked version stays in a crate's history forever, and the first real artifact
of three crates would not be their first version.

#### The preflight

The release job runs
[`scripts/check-crates-io-records.sh`](../scripts/check-crates-io-records.sh)
**before any packaging or publishing**. For every crate in the release set it
queries `/api/v1/crates/<name>` and:

- **permits** a crate with no record that the ledger names, reporting it as
  `PENDING … bootstrap pending` — the loop above handles it;
- **refuses** a crate with no record that the ledger does *not* name — an
  undocumented bootstrap requirement, which the loop would only skip if the
  ledger named it;
- **refuses** a ledgered crate that has a record at any version other than the
  one being released — a stale ledger entry (this section previously named
  `purrdf-datalog`, which has had a record since 2026-07-31, for a full cycle,
  because the list lived only in prose);
- **refuses** any record that is not locked to Trusted Publishing
  (`trustpub_only`), naming the setting — the step most easily forgotten
  between interleave runs.

Run it locally at any time; give it the version so the in-flight tolerance
applies:

```sh
PURRDF_RELEASE_VERSION=0.13.0 bash scripts/check-crates-io-records.sh
```

Only a literal 404 counts as missing: crates.io answers 403 to a default
`User-Agent`, so the script sends the same `purrdf-release/<version>` agent the
publish loop uses, retries transient statuses, and treats any other answer as a
hard stop rather than as "missing". Offline, `outstanding_bootstrap_claim` in
[`scripts/check-doc-claims.py`](../scripts/check-doc-claims.py) holds this
section — heading, crate names, publish-order ordinals and the anchor that links
here — to `PURRDF_UNBOOTSTRAPPED_CRATES` and `PURRDF_RELEASE_CRATES` on every
`make check`. *Membership* is only ever decided by the online gate: whether a
record exists is a fact about the registry, not about this tree.

What the preflight **cannot** see, so that a green run is not read as more than
it is: whether a Trusted Publisher entry exists for a crate and points at this
repository and workflow. crates.io has no public API for those entries. The OIDC
token the lane exchanges is scoped to the crates whose entries matched, and a
crate with no entry — including a record the token step just created whose
entry was not added before the run was re-run — fails at its **own**
`cargo publish` (`The provided access token is not valid for crate `<name>``),
after every crate ahead of it has been published: a loud stop under
`set -euo pipefail` with a partial publish that re-running the run resumes,
not a pre-publish refusal. The lock check is the closest a preflight gets,
because on crates.io the lock can only be enabled from a crate's settings page
once an entry exists.

`purrdf-python`, `purrdf-sparql-conformance`, `purrdf-cli`, and `purrdf-capi`
remain workspace crates, but they are not in this crates.io release lane.
`purrdf-python` is the PyPI extension package under `bindings/python`, the
conformance harness is an internal W3C fixture runner, `purrdf-cli` is the
native command-line package, and
the C ABI is a native artifact that should get a separate release lane if/when it
is shipped.

#### Verification is on, and why it was off

Each `cargo publish` — in the bootstrap script **and** in the tag lane's loop
([`scripts/publish-release-crates.sh`](../scripts/publish-release-crates.sh),
run by [`release-cargo.yaml`](../.github/workflows/release-cargo.yaml) under an
OIDC token) — **verifies**: cargo unpacks the `.crate` it
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

Two `--no-verify` remain, both deliberate. The pre-loop `cargo package` (the
whole workspace in the workflow, the ledger crates in the bootstrap) packages
every crate before *any* is published, so verification there is impossible by
construction in the workflow and pointless in the bootstrap, whose preflight
already proved every dependency resolves; it exists to find a packaging failure
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
crates.io record and is not in the bootstrap ledger, or has a record that is
not locked to Trusted Publishing (see [Outstanding
bootstrap](#outstanding-bootstrap-purrdf-cdt-purrdf-text-purrdf-geo)); that
check runs before packaging. It then publishes crates in dependency order,
skips any crate/version that already exists on crates.io (which keeps re-runs
safe after a partial publish), skips ledgered crates, and stops cleanly at the
first crate that depends on one — the interleave described there — resuming
with `gh run rerun <run-id>`.

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
