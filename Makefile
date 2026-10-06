# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

# REPOSITORY-GLOBAL: no `make: Entering directory '...'` banners, from ANY
# target — at any recursion depth ON GNU MAKE 4.4 OR NEWER.
#
# This is here because `make scale-corpus SCALE_MODE=pipe` puts a PAYLOAD on
# make's standard output, and make's directory banner lands on the same stream
# ahead of it as a corrupt first line a loader cannot parse. GNU make turns `-w`
# on automatically whenever it detects a sub-make (a nonzero inherited
# `MAKELEVEL`), so a wrapper that shells out to this lane inherits the
# corruption without asking for it.
#
# WHAT THIS LINE DOES AND DOES NOT BUY, precisely, because the difference used
# to be misstated here as immunity:
#
#   * GNU make >= 4.4: it cancels the automatic sub-make `-w`, so the bare
#     `make scale-corpus SCALE_MODE=pipe` is byte-exact at any `MAKELEVEL`.
#   * GNU make <= 4.3 (what ubuntu-24.04 runners ship): it does NOT. That make
#     decides `-w` at STARTUP from the inherited `MAKELEVEL`, before a single
#     line of makefile text is read, so this assignment arrives too late and the
#     banner is printed anyway. There is no makefile-internal fix on 4.3. The
#     target-specific form (`scale-corpus: MAKEFLAGS += ...`) is no better on
#     any version: make emits the banner before target-specific variables apply.
#
# So this line is a convenience that is real from 4.4, not the lane's guarantee.
# The guarantee lives in the idiom `docs/BENCHMARKS.md` documents for piping:
# `SCALE_MODE=pipe bash scripts/scale-corpus.sh | your-loader`, which runs no
# make at all and is therefore byte-exact on every version and at every
# `MAKELEVEL`. An explicit `make --no-print-directory` on the command line also
# works everywhere: the GNU make manual documents that option as cancelling the
# `-w` make turns on by itself in a sub-make, and it is parsed at startup
# alongside that `-w` rather than after it.
#
# The trade, stated plainly: every other target loses its directory banners too.
# Nothing in this file parses them, and the recipes that change directory say so
# themselves.
MAKEFLAGS += --no-print-directory

# Match Cargo's effective target directory, including `build.target-dir` from
# host/workspace configuration. An explicit environment or command-line value
# bypasses discovery.
ifeq ($(origin CARGO_TARGET_DIR), undefined)
CARGO_TARGET_DIR := $(shell cargo metadata --no-deps --format-version 1 2>/dev/null | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])' 2>/dev/null)
endif
ifeq ($(strip $(CARGO_TARGET_DIR)),)
$(error unable to resolve CARGO_TARGET_DIR; set it explicitly or ensure cargo metadata and python3 are available)
endif
CAPI_HEADER := crates/rdf-capi/include/purrdf.h
# The integration branch the non-Rust ratchet compares against: its BASE is the
# merge-base of this ref with HEAD. The pre-commit hook reads the same variable.
PURRDF_RATCHET_BASE ?= origin/main

.PHONY: help geodtest doctor metadata fmt hooks check test-shard simd-asm book book-samples book-pot book-po-update book-zh check-i18n check-issue-refs check-brand-casing check-spec-attribution changelog bump release-tags test doc bench bench-prepared-reuse bench-python scale-corpus columnar-oracle csvw-conformance csvw-oracle obographs-oracle projection-oracles pydantic-oracle linkml-oracle typescript-oracle graphql-oracle jsonschema-pattern-oracle pytest conformance iri-resolver-hygiene layer-hygiene helpers-hygiene serializer-rewind-hygiene terminal-hygiene thread-local-hygiene build-profile-hygiene rdf-core-hygiene python-binding-hygiene wasm wasm-test wasm-pkg wasm-pkg-test wasm-pkg-bench playground playground-smoke \
	capi-build capi-header capi-check capi-install test-gts-selected-blobs lint-gts-selected-blobs doc-gts-selected-blobs node-prerequisite binaryen-prerequisite cnschema-probe benchmark-acquire lubm watdiv miri

# The changelog generator is pinned so the detailed CHANGELOG.md history stays
# byte-reproducible across machines. GitHub uses a separately reviewed summary.
GIT_CLIFF_VERSION := 2.13.1

# binaryen (wasm-opt / wasm-dis) is pinned so the optimized npm artifact is
# byte-reproducible across machines. `wasm-pkg` hard-fails if the local wasm-opt
# does not report this version, exactly like the git-cliff pin above. The CI
# wasm-toolchain composite action reads this same value, so the pin lives in one
# place.
BINARYEN_VERSION := 130

# mdbook-i18n-helpers — the gettext preprocessor, the .pot extractor and the
# .po normalizer behind the zh-Hans book — is pinned so the extracted msgids
# and the rendered translation are reproducible across machines, exactly like
# git-cliff and binaryen above. `check-i18n` hard-fails unless
# `cargo install --list` reports this version (the binaries carry no version
# of their own, so the install record is the check); docs.yaml reads this same
# value, so the pin lives in one place.
MDBOOK_I18N_HELPERS_VERSION := 0.4.0

help: ## Show this help.
	@grep -E '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  %-18s %s\n", $$1, $$2}'

metadata: ## Regenerate + verify workspace metadata and generated artifacts.
	cargo metadata --no-deps --format-version 1 >/dev/null
	bash scripts/check-generated.sh --write
	$(MAKE) license-bundles

fmt: ## Auto-format the workspace.
	cargo fmt --all

hooks: ## Install the pre-commit and pre-merge-commit hooks: the quick subset of `make check`, run on the staged snapshot.
	git config core.hooksPath .githooks

check: node-prerequisite binaryen-prerequisite ## The full local gate: fmt, clippy, build, tests, hygiene.
	cargo fmt --all --check
	cargo fmt --manifest-path crates/jsonschema/tests/preserve_order_consumer/Cargo.toml --check
	cargo clippy --workspace --all-targets --locked -- -D warnings
	cargo clippy --manifest-path crates/jsonschema/tests/preserve_order_consumer/Cargo.toml --all-targets --locked -- -D warnings
	cargo check --workspace --lib --tests --locked
	cargo run -q --locked -p helper-census -- --no-features
	python3 scripts/check-toolchain-pin.py
	python3 scripts/check-toolchain-pin.py --self-test
	python3 scripts/check-gate-parity.py --self-test
	python3 scripts/check-gate-parity.py
	python3 scripts/check-stream-chunk.py --self-test
	python3 scripts/check-stream-chunk.py
	python3 scripts/check-build-profiles.py --self-test
	python3 scripts/check-build-profiles.py
	python3 scripts/check-iri-resolver-singleton.py
	python3 scripts/check-serializer-rewinds.py --self-test
	python3 scripts/check-serializer-rewinds.py
	cargo run -q --locked -p helper-census -- --python-binding-tests
	python3 scripts/check-terminal-predicates.py --self-test
	python3 scripts/check-terminal-predicates.py
	python3 scripts/check-thread-locals.py --self-test
	python3 scripts/check-thread-locals.py
	python3 scripts/check-shapes-parser-drops.py --self-test
	python3 scripts/check-shapes-parser-drops.py
	python3 scripts/check-licenses.py --self-test
	python3 scripts/check-licenses.py
	python3 scripts/fetch-locked-deps.py
	python3 scripts/package-licenses.py --self-test
	python3 scripts/capi-header.py --self-test
	python3 scripts/build-capi-bundle.py --self-test
	python3 scripts/build-private-wasm.py --self-test
	python3 bindings/python/purrdf_build_backend.py --self-test
	python3 scripts/package-licenses.py --check
	python3 scripts/check-banned-deps.py --self-test
	python3 scripts/check-banned-deps.py
	python3 scripts/check-banned-deps.py --ledger-complete
	python3 scripts/check-layers.py --self-test
	python3 scripts/check-layers.py
	cargo run -q --locked -p helper-census -- --self-test
	cargo run -q --locked -p helper-census -- --non-rust-ratchet --merge-base-with $(PURRDF_RATCHET_BASE) --target worktree
	python3 scripts/check-shared-helpers.py --self-test
	python3 scripts/check-shared-helpers.py
	python3 scripts/check-hash-domains.py --self-test
	python3 scripts/check-hash-domains.py
	python3 scripts/check-corpus-frozen.py
	bash scripts/check-generated.sh
	python3 scripts/check-issue-refs.py
	python3 scripts/check-brand-casing.py
	python3 scripts/check-spec-attribution.py --self-test
	python3 scripts/check-doc-claims.py
	python3 scripts/check-i18n-glossary.py
	python3 scripts/check-versions.py
	python3 scripts/check-release-notes.py --self-test
	python3 scripts/check-release-notes.py
	python3 scripts/check-publish-order.py
	bash scripts/check-crates-io-records.sh --self-test
	bash scripts/publish-release-crates.sh --self-test
	python3 scripts/publish-github-release.py --self-test
	python3 scripts/test_git_sync.py
	python3 scripts/publish-npm.py --self-test
	bash scripts/bootstrap-crates-io.sh --self-test
	python3 scripts/check-wasm-js-exports.py
	python3 scripts/check-entailment-surface.py
	python3 scripts/check-python-stub-parity.py
	python3 scripts/conformance-matrix.py --self-test
	python3 scripts/check-simd-asm.py --self-test
	python3 scripts/bench-suite-targets.py --self-test
	python3 scripts/cleanroom/transcript_audit.py --self-test
	python3 scripts/cleanroom/guard_hook.py --self-test
	python3 scripts/cleanroom/deny_settings.py --self-test
	python3 scripts/cleanroom/similarity.py --self-test
	python3 scripts/cleanroom/provenance.py --self-test
	python3 scripts/check-tracked-paths.py --self-test
	python3 scripts/check-tracked-paths.py
	python3 scripts/check-test-shards.py --self-test
	python3 scripts/check-test-shards.py
	python3 scripts/benchmark-acquire.py --self-test
	python3 scripts/watdiv-queries.py --offline-self-test
	python3 scripts/lubm-queries.py --offline-self-test
	python3 crates/text/tests/reference/bm25f.py --check
	cargo test --workspace --locked
	cargo test --manifest-path crates/jsonschema/tests/preserve_order_consumer/Cargo.toml --locked
	$(MAKE) rdf-core-hygiene
	$(MAKE) wasm

check-issue-refs: ## Reject issue-reference tokens (#NNN and tracker URLs) in comments, docs and corpora.
	python3 scripts/check-issue-refs.py

check-brand-casing: ## Reject bare lowercase 'purrdf' in prose (project is PurRDF in prose).
	python3 scripts/check-brand-casing.py

check-spec-attribution: ## Reject attributing a first-party extension (the quad template) to a SPARQL spec.
	python3 scripts/check-spec-attribution.py --self-test

check-i18n: ## Gate the zh-Hans translation: the glossary, then render it and run the prose gates + the SPARQL fence parser over the rendering (needs mdbook + the pinned mdbook-i18n-helpers).
	python3 scripts/check-i18n-glossary.py
	python3 scripts/check-i18n-render.py --self-test

changelog: ## Regenerate the deterministic CHANGELOG.md from conventional-commit history.
	@# git-cliff regenerates the WHOLE file from commit SUBJECTS and has no
	@# keep-region, so hand-authored release notes — in particular the prose that
	@# tells a consumer what to DO about a breaking change — are destroyed rather
	@# than merged. Refuse before the write, not after. Set
	@# CHANGELOG_ALLOW_REGENERATE=1 once the section has been preserved.
	@test -n "$(CHANGELOG_ALLOW_REGENERATE)" || python3 scripts/check-changelog-handwritten.py
	@command -v git-cliff >/dev/null 2>&1 || { \
		echo "ERROR: git-cliff not found — install the pinned version:"; \
		echo "  cargo install git-cliff --version $(GIT_CLIFF_VERSION) --locked --no-default-features"; \
		exit 1; \
	}
	@FOUND=$$(git-cliff --version | awk '{print $$2}'); \
		test "$$FOUND" = "$(GIT_CLIFF_VERSION)" || { \
			echo "ERROR: git-cliff version mismatch — found $$FOUND, expected $(GIT_CLIFF_VERSION):"; \
			echo "  cargo install git-cliff --version $(GIT_CLIFF_VERSION) --locked --no-default-features"; \
			exit 1; \
		}
	@VERSION=$$(python3 -c "import tomllib;print(tomllib.load(open('Cargo.toml','rb'))['workspace']['package']['version'])"); \
		git-cliff --config cliff.toml --tag "rust-v$$VERSION" --output CHANGELOG.md
	python3 scripts/check-issue-refs.py

bump: ## Set the crates.io/PyPI/npm version in lockstep (make bump VERSION=x.y.z).
	@test -n "$(VERSION)" || { echo "usage: make bump VERSION=x.y.z"; exit 1; }
	python3 scripts/set-version.py "$(VERSION)"
	@$(MAKE) --no-print-directory capi-header  # cbindgen derives PURRDF_MINOR from the crate version; a bump without this fails capi-check

release-tags: ## Cut + push rust-v/py-v/npm-v tags for VERSION after coherence checks (make release-tags VERSION=x.y.z).
	@test -n "$(VERSION)" || { echo "usage: make release-tags VERSION=x.y.z"; exit 1; }
	@test -z "$$(git status --porcelain)" || { echo "ERROR: working tree is dirty — commit the release bump + changelog first"; exit 1; }
	@branch=$$(git branch --show-current); test "$$branch" = "main" || { echo "ERROR: release tags must be cut from main (currently on $$branch)"; exit 1; }
	@python3 scripts/check-versions.py
	@python3 scripts/check-release-notes.py --version "$(VERSION)"
	@bash scripts/check-crates-io-records.sh --require-all
	@tree_version=$$(python3 -c "import tomllib;print(tomllib.load(open('Cargo.toml','rb'))['workspace']['package']['version'])"); \
		test "$$tree_version" = "$(VERSION)" || { echo "ERROR: VERSION=$(VERSION) does not match the tree version $$tree_version — run 'make bump VERSION=$(VERSION)' first"; exit 1; }
	@# Preserve the full versioned history alongside the reviewed short summary.
	@# Both must exist before the irreversible rust-v tag is pushed.
	@notes=$$(awk -v v="$(VERSION)" ' \
		$$0 == "## [" v "]" || index($$0, "## [" v "] ") == 1 { flag = 1; next } \
		/^## \[/ { flag = 0 } \
		flag { print } \
	' CHANGELOG.md); \
		test -n "$$(printf '%s' "$$notes" | tr -d '[:space:]')" || { echo "ERROR: CHANGELOG.md has no release-notes section for [$(VERSION)] — complete and commit the release notes before tagging"; exit 1; }
	@git fetch --quiet origin main
	@test "$$(git rev-parse HEAD)" = "$$(git rev-parse refs/remotes/origin/main)" || { echo "ERROR: main is not synchronized with origin/main"; exit 1; }
	@for tag in "rust-v$(VERSION)" "py-v$(VERSION)" "npm-v$(VERSION)"; do \
		! git rev-parse --quiet --verify "refs/tags/$$tag" >/dev/null || { echo "ERROR: local tag $$tag already exists"; exit 1; }; \
		! git ls-remote --exit-code --tags origin "refs/tags/$$tag" >/dev/null 2>&1 || { echo "ERROR: remote tag $$tag already exists"; exit 1; }; \
	done
	@echo "Running the complete release preflight before creating any tags..."
	@$(MAKE) check
	@$(MAKE) capi-check
	@$(MAKE) pytest
	@$(MAKE) python-release-check
	@$(MAKE) wasm-pkg-test
	@$(MAKE) capi-bundle
	@test -z "$$(git status --porcelain)" || { echo "ERROR: full release preflight changed the working tree"; exit 1; }
	@branch=$$(git branch --show-current); test "$$branch" = "main" || { echo "ERROR: branch changed during release preflight (currently on $$branch)"; exit 1; }
	@git fetch --quiet origin main
	@test "$$(git rev-parse HEAD)" = "$$(git rev-parse refs/remotes/origin/main)" || { echo "ERROR: main moved during release preflight; update main and rerun"; exit 1; }
	@for tag in "rust-v$(VERSION)" "py-v$(VERSION)" "npm-v$(VERSION)"; do \
		! git rev-parse --quiet --verify "refs/tags/$$tag" >/dev/null || { echo "ERROR: local tag $$tag appeared during release preflight"; exit 1; }; \
		! git ls-remote --exit-code --tags origin "refs/tags/$$tag" >/dev/null 2>&1 || { echo "ERROR: remote tag $$tag appeared during release preflight"; exit 1; }; \
	done
	git tag "rust-v$(VERSION)"
	git tag "py-v$(VERSION)"
	git tag "npm-v$(VERSION)"
	git push --atomic origin "rust-v$(VERSION)" "py-v$(VERSION)" "npm-v$(VERSION)"
	@echo "OK: pushed rust-v$(VERSION), py-v$(VERSION), npm-v$(VERSION)"

binaryen-prerequisite: ## Require the pinned structural wasm validator used by wasm-link.
	@test "$$(wasm-opt --version 2>/dev/null | sed -n 's/.*version \([0-9][0-9]*\).*/\1/p')" = "$(BINARYEN_VERSION)" || { \
		echo "ERROR: wasm-opt $(BINARYEN_VERSION) is required; install the pinned Binaryen toolchain." >&2; exit 1; \
	}

node-prerequisite: ## Require Node for the native Unicode ECMAScript conformance oracle.
	@command -v node >/dev/null 2>&1 && node --version >/dev/null 2>&1 || { \
		echo "ERROR: Node.js is required for the native Unicode ECMAScript conformance oracle; install Node.js and put node on PATH." >&2; \
		exit 1; \
	}

test: node-prerequisite binaryen-prerequisite ## Run the workspace test suite.
	cargo test --workspace --locked

test-shard: node-prerequisite ## Run one CI shard of `make test` (SHARD=lib|doc|integration-1..4).
	@case "$(SHARD)" in \
		lib) $(MAKE) binaryen-prerequisite || exit $$?; set -x; cargo test --workspace --exclude purrdf-python --locked --lib --bins ;; \
		doc) set -x; cargo test --workspace --locked --doc \
			&& cargo build --workspace --locked --examples --profile test \
			&& cargo test --workspace --locked --example graphql_oracle_fixture --example typescript_oracle_fixture --example text_relevance ;; \
		integration-1) set -x; cargo test --workspace --locked --test '[a-d]*' -- --exact --skip c_abi_smoke ;; \
		integration-2) set -x; cargo test --workspace --locked --test '[e-o]*' ;; \
		integration-3) set -x; cargo test --workspace --locked --test '[p-r]*' ;; \
		integration-4) set -x; cargo test --workspace --locked --test '[!a-r]*' ;; \
		*) echo "FAIL: unknown SHARD '$(SHARD)'; expected lib, doc or integration-1..4" >&2; exit 1 ;; \
	esac

lint-gts-selected-blobs: ## Lint the selected native-import production and test surfaces only.
	cargo clippy -p purrdf-gts --lib --test bounded_keyed_blobs --locked -- -D warnings
	cargo clippy -p purrdf-rdf --lib --test gts_selected_blobs --locked -- -D warnings
	cargo clippy -p purrdf-shapes --lib --locked -- -D warnings

doc-gts-selected-blobs: ## Check the native-import and shared shape-dataset public API documentation.
	RUSTDOCFLAGS="-D warnings" cargo doc -p purrdf-gts -p purrdf-rdf -p purrdf-shapes --no-deps --locked

test-gts-selected-blobs: ## Check bounded selected-blob import and native scope contracts only.
	cargo test -p purrdf-gts --lib codec::tests:: --locked
	cargo test -p purrdf-gts --test bounded_keyed_blobs --locked
	cargo test -p purrdf-rdf --lib gts_import_sink::tests:: --locked
	cargo test -p purrdf-rdf --test gts_selected_blobs --locked
	cargo test -p purrdf-shapes --test shared_shapes_dataset --locked

doc: ## Build docs for the 32 publishable crates with rustdoc warnings denied.
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --exclude purrdf-capi --exclude purrdf-python --exclude purrdf-sparql-conformance --exclude purrdf-cli

book-samples: ## Regenerate deterministic SVG visualization samples embedded in The PurRDF Book.
	@set -eu; \
		target=docs/book/src/assets/visualization; \
		tmp=$$(mktemp -d docs/book/src/assets/.visualization.XXXXXX); \
		previous="$$tmp.previous"; \
		cleanup() { \
			status=$$?; \
			if [ -d "$$previous" ] && [ ! -d "$$target" ]; then mv "$$previous" "$$target"; fi; \
			rm -rf "$$tmp" "$$previous"; \
			exit $$status; \
		}; \
		trap cleanup EXIT; \
		cargo run -p purrdf-rdf --example viz_samples --locked -- "$$tmp" --svg-only; \
		if [ -d "$$target" ]; then mv "$$target" "$$previous"; fi; \
		mv "$$tmp" "$$target"; \
		rm -rf "$$previous"; \
		trap - EXIT

book: book-samples ## Build The PurRDF Book (mdBook user guide) into docs/book/book/.
	mdbook build docs/book

book-zh: ## Build the zh-Hans book into docs/book/book/zh-Hans/ (after `book`; search off — see book.toml).
	MDBOOK_BOOK__LANGUAGE=zh-Hans MDBOOK_OUTPUT__HTML__SEARCH__ENABLE=false mdbook build -d docs/book/book/zh-Hans docs/book

book-pot: ## Extract the translation template docs/book/po/messages.pot (ignored) from the English book.
	MDBOOK_OUTPUT='{"xgettext": {"pot-file": "messages.pot"}}' mdbook build -d docs/book/po docs/book

book-po-update: book-pot ## Refresh docs/book/po/zh-Hans.po against the current English source (msgmerge; fuzzy/obsolete entries render as English until retranslated).
	msgmerge --quiet --update --backup=none docs/book/po/zh-Hans.po docs/book/po/messages.pot

bench-prepared-reuse: ## Measure cold/warm preparation and prepared execution on O3/full-LTO code (report-only).
	CARGO_PROFILE_RELEASE_OPT_LEVEL=3 CARGO_PROFILE_RELEASE_LTO=fat \
	CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_INCREMENTAL=false \
	CARGO_PROFILE_RELEASE_DEBUG=false \
	cargo bench --locked --profile release -p purrdf-sparql-eval --bench prepared_reuse -- $(BENCH_ARGS)

bench: ## Run the purrdf_testkit::bench suites (report-only; never a gate).
	cargo bench -p purrdf-gts -p purrdf-core -p purrdf-columnar -p purrdf-rdf -p purrdf-json -p purrdf-sparql-eval -p purrdf-geo -p purrdf-geo-kernel -p purrdf-text -p purrdf-shapes -p purrdf-wasm -p purrdf-entail -p purrdf-lex -p purrdf-iri -p purrdf-xsd -p purrdf-sparql-algebra -p purrdf-sparql-results -p purrdf-hash-conformance -p purrdf-deflate -p purrdf-jsonschema

# HOW A LANE KNOB REACHES ITS SCRIPT: as environment bytes, unparsed.
#
# A `make` recipe line is handed to `/bin/sh`, so a knob interpolated into one
# gets a round of SHELL evaluation the operator never asked for. That is not
# theoretical: `make scale-corpus 'SCALE_MANIFEST=/tmp/`touch /tmp/pwned`m.json'`
# ran the backtick and then wrote the manifest to `/tmp/m.json` — a DIFFERENT
# PATH than the one requested — and still exited 0. Quoting the interpolation
# does not fix it: a double-quoted `sh` string still expands `` ` `` and `$`, and
# a value containing a `"` becomes a raw `sh: unexpected EOF` instead of a lane
# diagnostic. And `make` has a second expansion of its own — `$` in a
# command-line variable value is `make`'s OWN variable syntax, so
# `SCALE_MANIFEST=m$x.json` silently became `m.json`.
#
# `lane-env` closes both. `override X := $(value X)` freezes the bytes the
# operator actually typed, before `make` expands anything in them; `export X`
# hands exactly those bytes to the child's environment, where no shell parses
# them. The lane scripts already read every one of these knobs from the
# environment (`${SCALE_OUT:-}` and friends), so a path is now taken literally,
# byte for byte, or the lane hard-fails saying which bytes it used.
#
# The one deliberate exception is `SCALE_SINK`, which is DOCUMENTED AS A COMMAND
# and therefore must still reach a shell. It does so at exactly one place,
# inside scripts/scale-corpus.sh, which syntax-checks it up front and fails with
# a lane diagnostic if it cannot be run — never a bare `sh` error, never a
# silent success.
define lane-env
override $(1) := $$(value $(1))
export $(1)
endef

# The scale-corpus lane's knobs. Overridable exactly like BENCH_ARGS above:
# `make scale-corpus SCALE_QUADS=10000000 SCALE_SHARDS=16`. The defaults are a
# small streamed run because the profile's density rises with the entity index
# space (~177 bytes per row at a 10^6-entity space, ~183 at 10^10) — a
# 10^10-row run over a matching 10^10-entity space is ~1.83 TB, which is
# streamed, not stored. SCALE_MODE=files is the only mode that writes
# anything, and it refuses to run without SCALE_OUT.
SCALE_QUADS ?= 1000000
SCALE_IRIS ?= 100000
SCALE_SEED ?= 1592642302
SCALE_SHARDS ?= 8
SCALE_MODE ?= stream
SCALE_OUT ?=
SCALE_SINK ?=
SCALE_MANIFEST ?=
# SCALE_BIN names the executable that certifies the bytes, exactly as LUBM_BIN
# and WATDIV_BIN do for their lanes, so it belongs in `lane-env` more than any
# other knob here. It was the one knob the script read that the `Makefile` never
# listed, and the omission was the whole defect `lane-env` exists to stop:
# `make scale-corpus 'SCALE_BIN=/tmp/bin/de$$xcoy'` had `$$x` expanded away by
# `make` and the lane then EXECUTED `/tmp/bin/decoy` — a different binary than
# the operator named, with exit 0.
SCALE_BIN ?=

$(foreach knob,SCALE_QUADS SCALE_IRIS SCALE_SEED SCALE_SHARDS SCALE_MODE SCALE_OUT SCALE_SINK SCALE_MANIFEST SCALE_BIN,$(eval $(call lane-env,$(knob))))

scale-corpus: ## Generate the deterministic scale corpus across shards (streams and retains nothing by default; report-only, never a gate). See docs/BENCHMARKS.md.
	@bash scripts/scale-corpus.sh

columnar-oracle: ## Verify production Parquet files through the dev-only DuckDB oracle.
	bash scripts/check-columnar-oracle.sh

csvw-conformance: ## Run every pinned W3C CSVW RDF and validation manifest case.
	cargo test -p purrdf-rdf --test csvw_w3c --locked

csvw-oracle: ## Validate canonical CSVW output with the locked independent csvw 4.1.0 implementation.
	bash scripts/check-csvw-oracle.sh

obographs-oracle: ## Validate deterministic output against the pinned official OBO Graphs 0.3.2 schema.
	bash scripts/check-obographs-schema-oracle.sh

projection-oracles: csvw-conformance csvw-oracle obographs-oracle ## Run graph/tabular conformance and independent corruption oracles.

pydantic-oracle: ## Execute emitted Pydantic v2 models and compare model_json_schema() with CompiledSchema.
	uv sync --project bindings/python --locked --no-install-project
	uv run --project bindings/python --no-sync python crates/shapes/tests/pydantic_oracle.py

linkml-oracle: ## Validate emitted LinkML through the locked official 1.11 toolchain.
	uv sync --project bindings/python --locked --no-install-project
	uv run --project bindings/python --no-sync python crates/shapes/tests/linkml_oracle.py

typescript-oracle: ## Compile emitted declarations with TypeScript 7.0 and compare assignability with CompiledSchema.
	npm --prefix crates/rdf-wasm/js ci --ignore-scripts --no-audit --no-fund
	node crates/shapes/tests/typescript_oracle.mjs

graphql-oracle: ## Validate emitted SDL and variable coercion with locked GraphQL.js and purrdf-jsonschema.
	npm --prefix crates/rdf-wasm/js ci --ignore-scripts --no-audit --no-fund
	node crates/shapes/tests/graphql_oracle.mjs

jsonschema-pattern-oracle: node-prerequisite ## Re-ask JavaScript's RegExp for every frozen purrdf-jsonschema pattern verdict; fails if any answer moved.
	node crates/jsonschema/tests/pattern_oracle.mjs

bench-python: ## Compare the rdflib compat shim vs. real rdflib (report-only; NOT a test gate). See docs/BENCHMARKS.md.
	cd bindings/python && uv run maturin develop && uv run python benchmarks/bench_compat.py

pytest: ## Build the native module + run the Python binding test suite (own gate, NOT part of `check`).
	cargo run -q --locked -p helper-census -- --python-binding-tests
	cd bindings/python && uv sync --locked --group dev --reinstall-package purrdf && uv run --locked pytest tests
	cd bindings/python && PURRDF_TEST_REQUIRE_EXACT=1 uv run --locked cargo test --manifest-path ../../Cargo.toml --locked -p purrdf-cli --test python_empty_graphs -- --ignored --exact installed_empty_graph_modes

miri: ## Check SmallVec storage and BLAKE3 streaming under Miri (own lane, NOT part of `check`).
	@# `purrdf_core::SmallVec` keeps its inline elements in uninitialised
	@# storage and moves them with raw pointer copies; its tests pin ownership
	@# (no double drop, no leak, no uninitialised read) but only an interpreter
	@# can see an aliasing violation or an out-of-bounds offset that happens to
	@# work natively. Each run is a distinct model: the default Stacked Borrows,
	@# Tree Borrows, strict provenance (no integer-to-pointer casts), and a
	@# 32-bit target, where `usize` arithmetic and layout differ.
	cargo miri test -p purrdf-core small
	MIRIFLAGS=-Zmiri-tree-borrows cargo miri test -p purrdf-core small
	MIRIFLAGS=-Zmiri-strict-provenance cargo miri test -p purrdf-core small
	cargo miri test -p purrdf-core small --target i686-unknown-linux-gnu
	@# Bounded streaming vectors cover buffer/tree boundaries and snapshots.
	@# The scalar rotation uses Rust under Miri; native lowering is checked separately.
	MIRIFLAGS=-Zmiri-strict-provenance cargo miri test --locked -p purrdf-hash-conformance --test blake3 streaming_boundary_answers

# CI runs the matrix split across runners through CONFORMANCE_ARGS: one
# `--shard NAME --emit-results FILE` per shard, then one `--from-results DIR` that
# judges the whole matrix exactly as a single run does. Empty is the full run.
CONFORMANCE_ARGS ?=
conformance: ## Umbrella conformance matrix: native Rust W3C suites + the Python rdflib drop-in gate, one scoreboard (see docs/CONFORMANCE.md).
	python3 scripts/conformance-matrix.py $(CONFORMANCE_ARGS)

iri-resolver-hygiene: ## Prove the resolver ring-fence: RFC 3986 reference resolution only in crates/iri/src.
	python3 scripts/check-iri-resolver-singleton.py

layer-hygiene: ## Prove every first-party crate edge is one layers.toml allows, and the table is exact.
	python3 scripts/check-layers.py --self-test
	python3 scripts/check-layers.py

helpers-hygiene: ## Prove helpers-ledger.toml holds (one home per job, no forbidden copy, no stale exemption, no cross-crate #[path]) and every hash domain is a unique, prefix-free purrdf_hash::Domain.
	cargo run -q --locked -p helper-census -- --self-test
	python3 scripts/check-shared-helpers.py --self-test
	python3 scripts/check-shared-helpers.py
	python3 scripts/check-hash-domains.py --self-test
	python3 scripts/check-hash-domains.py

serializer-rewind-hygiene: ## Prove no serializer takes back output it already produced.
	python3 scripts/check-serializer-rewinds.py --self-test
	python3 scripts/check-serializer-rewinds.py

python-binding-hygiene: ## Prove no Rust test module hides in the PyO3 extension crate (it would never compile or run).
	cargo run -q --locked -p helper-census -- --self-test
	cargo run -q --locked -p helper-census -- --python-binding-tests

terminal-hygiene: ## Prove no scanner decides a token boundary with a Unicode property.
	python3 scripts/check-terminal-predicates.py --self-test
	python3 scripts/check-terminal-predicates.py

thread-local-hygiene: ## Prove every thread_local! in crates/ is in the wasm interleaving ledger with its reason, and nothing stale is.
	python3 scripts/check-thread-locals.py --self-test
	python3 scripts/check-thread-locals.py

build-profile-hygiene: ## Prove the gate really compiles at opt-level 3 with debug-assertions and overflow-checks on.
	python3 scripts/check-build-profiles.py --self-test
	python3 scripts/check-build-profiles.py

rdf-core-hygiene: ## Prove the kernel ring-fence: no PyO3 in purrdf-core; the root and the ring-fenced crates depend only on what layers.toml lists.
	@tree=$$(cargo tree --color never -p purrdf-core --edges normal -f "{p}") || { echo "FAIL: cargo tree errored"; exit 1; }; \
	if echo "$$tree" | grep -q 'pyo3 v'; then \
		echo "FAIL: purrdf-core pulls PyO3 as a NORMAL dependency"; \
		echo "$$tree" | grep 'pyo3 v'; exit 1; \
	fi; \
	echo "OK: purrdf-core has no PyO3 normal dependency"
	@# The root (`root` in layers.toml) must have zero runtime dependencies, and
	@# every crate whose row carries `external` may depend only on its row's
	@# `deps` and `external`, counting every target's normal edges.
	python3 scripts/check-layers.py --ring-fence

cnschema-probe: ## Reproduce the pinned cnSchema 4.0 round-trip evidence (fetches by digest; not a CI gate).
	python3 scripts/cnschema-probe.py --self-test
	python3 scripts/cnschema-probe.py

# The LUBM and WatDiv artifacts are NEVER vendored: the LUBM generator is
# GPL-2.0-or-later, its ontology and query file publish no licence grant at all,
# and WatDiv grants use-with-citation rather than redistribution. They are
# fetched by digest into target/bench-artifacts/ at the moment of use. Run
# `python3 scripts/benchmark-acquire.py --list` to read each artifact's terms.
benchmark-acquire: ## Fetch the pinned LUBM and WatDiv comparison-workload artifacts by digest into target/ (network; nothing is vendored; not a CI gate).
	python3 scripts/benchmark-acquire.py --self-test
	python3 scripts/benchmark-acquire.py

# The LUBM comparison lane's knobs. Overridable exactly like SCALE_* above:
# `make lubm LUBM_UNIVERSITIES=5`. The default is ONE university (~103k triples),
# which is small enough to run in seconds and is the size whose per-query answers
# the LUBM literature publishes, so a first run can be checked against it.
#
# LUBM_DOC_BASE is the base each generated document's own two header triples
# resolve against. It has a default because leaving it unset would silently embed
# the scratch directory's `file://` path and destroy reproducibility; example.org
# is RFC 2606's reserved documentation domain and this repository's fixture
# convention, standing in for a publication IRI a local corpus does not have.
LUBM_UNIVERSITIES ?= 1
LUBM_SEED ?= 0
LUBM_INDEX ?= 0
LUBM_ONTO ?= http://swat.cse.lehigh.edu/onto/univ-bench.owl
LUBM_DOC_BASE ?= http://example.org/lubm/
LUBM_ENTAIL_SLICE ?= 3000
LUBM_OUT ?= target/lubm
LUBM_BIN ?=

# Environment bytes, not a shell assignment prefix — see `lane-env` above. A
# backtick in `LUBM_OUT` used to run and the lane then worked in a directory
# nobody named. None of these knobs is a command, so none of them reaches a
# shell at all.
$(foreach knob,LUBM_UNIVERSITIES LUBM_SEED LUBM_INDEX LUBM_ONTO LUBM_DOC_BASE LUBM_ENTAIL_SLICE LUBM_OUT LUBM_BIN,$(eval $(call lane-env,$(knob))))

lubm: ## Run the LUBM comparison workload end to end - acquire, generate, convert through the purrdf CLI, and run the 14 queries per entailment regime (report-only, never a gate). See docs/BENCHMARKS.md.
	@bash scripts/lubm-lane.sh

# The WatDiv comparison lane's knobs, in the same style as LUBM_* above:
# `make watdiv WATDIV_SEED=7`. The default dataset is upstream's frozen 10M output
# (~10.9M triples), which is the only scale pinned by digest: WatDiv's generator
# seeds itself from the wall clock and has no seed flag, so a dataset is
# reproducible only as a frozen OUTPUT, never as a generation run. The generator is
# never built and never run here.
#
# WATDIV_SEED fixes the query set. The 20 published templates carry `%vN%`
# placeholders that something must fill in, and upstream's own instantiator is
# time-seeded and irreproducible; this lane chooses deterministically from the
# frozen dataset instead. A DIFFERENT SEED IS A DIFFERENT WORKLOAD, so the seed is
# reported next to every number it governs.
WATDIV_SCALE ?= 10M
WATDIV_SEED ?= 0
WATDIV_OUT ?= target/watdiv
WATDIV_BIN ?=

# Environment bytes, not a shell assignment prefix — see `lane-env` above.
$(foreach knob,WATDIV_SCALE WATDIV_SEED WATDIV_OUT WATDIV_BIN,$(eval $(call lane-env,$(knob))))

watdiv: ## Run the WatDiv comparison workload end to end - acquire the frozen dataset, instantiate the 20 templates deterministically, load through the purrdf CLI, and run them (pure BGP, no entailment; report-only, never a gate). See docs/BENCHMARKS.md.
	@bash scripts/watdiv-lane.sh

# `purrdf-bench` is unpublished tooling rather than a release crate, and it is in
# this list anyway: its library half documents itself as portable, and a
# portability claim that no gate builds is carried by whoever last ran the build
# by hand. Only the LIBRARY is built here (this recipe passes `--lib`); the
# `bench-corpus` binary is std-only — files, process arguments, an exit code —
# and stays deliberately out of the wasm surface.
wasm: ## Build the release crates for wasm32-unknown-unknown (SKIP locally if target absent; CI hard-fails).
	@if rustup target list --installed 2>/dev/null | grep -qx wasm32-unknown-unknown; then \
		cargo build --locked --release --target wasm32-unknown-unknown --lib \
			-p purrdf-events -p purrdf-lex -p purrdf-iri -p purrdf-xsd -p purrdf-cdt -p purrdf-jsonschema -p purrdf-hash -p purrdf-deflate -p purrdf-stack -p purrdf-ed25519 -p purrdf-gts -p purrdf-core -p purrdf-columnar \
			-p purrdf-datalog \
			-p purrdf-sparql-algebra -p purrdf-sparql-results -p purrdf-sparql-eval -p purrdf-hnsw \
			-p purrdf-rdf -p purrdf-markdown -p purrdf-json -p purrdf-slice -p purrdf-shapes -p purrdf-shex -p purrdf-entail \
			-p purrdf-geo -p purrdf-geo-kernel -p purrdf-text -p purrdf-retrieval \
			-p purrdf-validate -p purrdf -p purrdf-wasm \
			-p purrdf-bench; \
	elif [ -n "$${CI:-}" ]; then \
		echo "FAIL: wasm32-unknown-unknown target absent in CI"; exit 1; \
	elif ! command -v rustup >/dev/null 2>&1; then \
		echo "SKIP: rustup is not on PATH, so the wasm32 target cannot be detected OR installed."; \
		echo "      This is NOT the same as 'target not installed': with no rustup, rust-toolchain.toml's"; \
		echo "      pinned toolchain is also unenforced locally, and this gate is silently inert."; \
		echo "      Run 'make doctor' for the full list of pins this machine does not enforce."; \
	else \
		echo "SKIP: wasm32-unknown-unknown target not installed — 'rustup target add wasm32-unknown-unknown' to enable"; \
	fi

doctor: ## Report which build pins this machine actually enforces (never gates; run it when a gate SKIPs).
	@# Every "SKIP" a gate prints is a claim that something was not checked, and the
	@# reasons are not interchangeable: a missing TARGET is one command away, while a
	@# missing `rustup` means `rust-toolchain.toml` is inert and the toolchain running
	@# your gate is whatever `cargo` happens to resolve to. Both used to print the same
	@# line, because `rustup ... 2>/dev/null` swallows "command not found" and lands in
	@# the same branch as "target absent". This target states the difference out loud.
	@echo "PurRDF environment doctor — what this machine enforces"
	@echo
	@printf 'toolchain pin (rust-toolchain.toml): '
	@grep -oE 'channel = "[^"]+"' rust-toolchain.toml 2>/dev/null | head -1 | cut -d'"' -f2 || echo "(none declared)"
	@printf 'rustup:                              '
	@if command -v rustup >/dev/null 2>&1; then \
		rustup --version 2>/dev/null | head -1; \
	else \
		echo "ABSENT — rust-toolchain.toml is NOT enforced on this machine."; \
		echo '                                     `cargo`/`rustc` resolve to whatever is on PATH, so the'; \
		echo "                                     nightly channel is honoured only in CI, and \`make wasm\`"; \
		echo "                                     cannot detect or install the wasm32 target (it SKIPs)."; \
	fi
	@printf 'active rustc:                        '
	@rustc --version 2>/dev/null || echo "ABSENT"
	@printf 'wasm32-unknown-unknown target:       '
	@if ! command -v rustup >/dev/null 2>&1; then \
		echo "UNKNOWN (no rustup to ask) — \`make wasm\` will SKIP"; \
	elif rustup target list --installed 2>/dev/null | grep -qx wasm32-unknown-unknown; then \
		echo "installed — \`make wasm\` runs for real"; \
	else \
		echo "NOT installed — \`make wasm\` SKIPs; 'rustup target add wasm32-unknown-unknown'"; \
	fi
	@printf 'wasm-bindgen CLI:                    '
	@PIN=$$(sed -n 's/^wasm-bindgen = "=\([0-9][0-9.]*\)"$$/\1/p' Cargo.toml); \
	if ! command -v wasm-bindgen >/dev/null 2>&1; then \
		echo "absent — \`make wasm-test\` SKIP; install wasm-bindgen-cli $$PIN"; \
	else \
		FOUND=$$(wasm-bindgen --version | sed -n 's/^wasm-bindgen \([0-9][0-9.]*\).*$$/\1/p'); \
		if [ "$$FOUND" = "$$PIN" ]; then \
			echo "$$FOUND on PATH — the wasm32 test runner (scripts/wasm-test-runner.sh) runs for real"; \
		else \
			echo "$$FOUND on PATH, but Cargo.toml pins $$PIN — the wasm32 test runner refuses it; install wasm-bindgen-cli $$PIN"; \
		fi; \
	fi
	@printf 'node:                                '
	@command -v node >/dev/null 2>&1 && node --version || echo "absent — make check and make test FAIL; the wasm test harness SKIPs"
	@printf 'cargo build directory:               '
	@echo "$(CARGO_TARGET_DIR)"
	@echo
	@echo "A SKIP is not a pass. In CI every line above is a hard failure instead."

# The SIMD asm evidence gate: seven release builds (x86_64 baseline, x86-64-v3,
# x86-64-v4, aarch64, aarch64 neoverse-v1, wasm32, wasm32 +simd128) with
# `--emit=asm`, then every site in scripts/simd-asm-manifest.toml is counted in the
# emitted functions. It needs the aarch64 and wasm32 standard libraries (the audited
# graph is pure Rust, so no C toolchain); a missing one is a failure here, never a
# skip. Too slow for `check`, which runs only its `--self-test`.
#
# `--doc` adds the audit document's checks: every manifest site is a row of
# docs/design/purrdf-simd.md and every function row has a manifest site, its
# generated count cells equal this measurement, and every workspace member and bench
# file is covered. `--doc` hard-fails when the document is missing; it never skips.
# `python3 scripts/check-simd-asm.py --write-doc` regenerates the count cells.
#
# CI runs the same gate split across runners through SIMD_ASM_ARGS: one
# `--config NAME --report FILE` per configuration, then one
# `--merge-reports DIR` that requires seven matching successful reports and
# checks the document against them. Empty measures the complete matrix.
# Geographic laws additionally require complete identical records on native,
# portable wasm and SIMD wasm. This bounded corpus has its own runner gate.
geo-determinism: ## Execute frozen geometry/geodesy bytes on native and both wasm paths.
	cargo run --locked -p purrdf-geo-kernel --example qualify_determinism

# Karney's public GeodTest (CC0, https://zenodo.org/records/32156, 500,000
# rows) is not vendored. Point GEODTEST at GeodTest.dat; the runner refuses
# any input whose SHA-256 differs from the published file's.
GEODTEST ?= GeodTest.dat
GEODTEST_SHA256 := c1cabdddbcd7d5cfc6e6111db4608fa55be292b15ba2c5bcd6372a178848c692
geodtest: ## Qualify distance, inverse and direct on all 500,000 GeodTest rows, digest-checked (own gate, NOT part of `check`).
	cargo run --locked --release -p purrdf-geo-kernel --example geodesic_corpus -- 500000 10000000 all portable $(GEODTEST_SHA256) < $(GEODTEST)

SIMD_ASM_ARGS ?=
simd-asm: ## Count the vector work in emitted asm on seven target configurations (own gate, NOT part of `check`).
	python3 scripts/check-simd-asm.py --doc $(SIMD_ASM_ARGS)

wasm-test: ## Execute WASM dispatch, SIMD kernels, shadow-stack and host-interface probes in Node.
	@# Native Rust owns semantic/conformance corpora. This lane executes only the
	@# named WASM behaviors documented in docs/WASM_TESTING.md: actual dispatch,
	@# SIMD kernels, shadow-stack floors and host clock/storage refusal. Full test
	@# targets remain registered under cargo test --workspace.
	@# Runner preflight observes planted panic, refused flags and sealed host reads.
	@# +simd128 is target codegen, not a Cargo feature. Target-scoped flags replace
	@# build.rustflags and are ignored when RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS is
	@# set, so fold/unset RUSTFLAGS, restate -D warnings, and refuse encoded flags.
	@if [ -n "$${CARGO_ENCODED_RUSTFLAGS:-}" ]; then \
		echo "FAIL: CARGO_ENCODED_RUSTFLAGS is set; Cargo would ignore the +simd128 run's target-scoped flags and run the baseline build twice"; exit 1; \
	fi
	@if ! rustup target list --installed 2>/dev/null | grep -qx wasm32-unknown-unknown; then \
		if [ -n "$${CI:-}" ]; then echo "FAIL: wasm32-unknown-unknown target absent in CI"; exit 1; fi; \
		if ! command -v rustup >/dev/null 2>&1; then \
			echo "SKIP: rustup is not on PATH — the wasm32 target cannot be detected or installed ('make doctor')"; \
		else \
			echo "SKIP: wasm32-unknown-unknown target not installed — 'rustup target add wasm32-unknown-unknown' to enable"; \
		fi; \
	elif ! command -v wasm-bindgen >/dev/null 2>&1; then \
		if [ -n "$${CI:-}" ]; then echo "FAIL: the wasm-bindgen CLI is absent in CI"; exit 1; fi; \
		echo "SKIP: the wasm-bindgen CLI is not on PATH — install wasm-bindgen-cli $$(sed -n 's/^wasm-bindgen = \"=\([0-9][0-9.]*\)\"$$/\1/p' Cargo.toml) to enable"; \
	elif ! command -v node >/dev/null 2>&1; then \
		if [ -n "$${CI:-}" ]; then echo "FAIL: node absent in CI"; exit 1; fi; \
		echo "SKIP: node not on PATH — the wasm test harness runs the module in Node"; \
	else \
		bash scripts/check-wasm-test-runner.sh \
		&& export PURRDF_TEST_REQUIRE_EXACT=1 \
		&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-sparql-eval --test knn_wasm_reassociated -- --exact \
				the_reassociated_path_is_the_one_this_build_was_made_for \
		&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-sparql-eval --test stack_refusal -- --exact \
				prepared_evaluation_refuses_the_actual_smaller_stack_without_poisoning_the_caller \
		&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-core --test model_traits -- --exact \
				owned_term_walks_stay_inside_the_wasm_shadow_stack_floor \
		&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-hnsw --test wasm_reassociated -- --exact \
				an_image_recorded_on_another_wasm_path_is_refused_by_name \
				the_image_records_the_path_and_shape_this_build_was_made_for \
		&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-hash-conformance --test blake3 -- --exact \
				required_backends_are_available \
		&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-testkit --test bench -- --exact \
				the_store_options_are_refused_on_wasm32 \
				a_measured_run_reports_its_estimates \
		&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-stack --test on_stack -- --exact \
				an_in_floor_request_runs \
				a_scoped_in_floor_request_borrows_the_caller_s_locals \
				an_over_floor_request_is_refused \
				an_over_floor_scoped_request_is_refused \
		&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-deflate --test deflate_conformance -- --exact \
				selected_backend_is_reported \
		&& env -u RUSTFLAGS \
			CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="$${RUSTFLAGS:-} $${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128" \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-sparql-eval --test knn_wasm_reassociated -- --exact \
				the_reassociated_path_is_the_one_this_build_was_made_for \
				the_reassociated_distance_is_within_the_error_bound_of_the_exact_one \
		&& env -u RUSTFLAGS \
			CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="$${RUSTFLAGS:-} $${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128" \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-hnsw --test wasm_reassociated -- --exact \
				an_image_recorded_on_another_wasm_path_is_refused_by_name \
				the_image_records_the_path_and_shape_this_build_was_made_for \
		&& env -u RUSTFLAGS \
			CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="$${RUSTFLAGS:-} $${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128" \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-hash-conformance --test hex -- --exact \
				every_path_matches_portable \
		&& env -u RUSTFLAGS \
			CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="$${RUSTFLAGS:-} $${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128" \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-hash-conformance --test blake3 -- --exact \
				required_backends_are_available \
				random_inputs_cover_irregular_trees_and_alignment \
		&& env -u RUSTFLAGS \
			CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="$${RUSTFLAGS:-} $${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128" \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-core --test csv_scan_wasm -- --exact \
				every_kernel_agrees_with_the_per_byte_scan_at_every_alignment_and_length \
				every_kernel_agrees_with_the_per_byte_scan_over_seeded_inputs \
				the_target_explicit_kernel_is_among_those_compared \
		&& env -u RUSTFLAGS \
			CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$(CURDIR)/scripts/wasm-test-runner.sh \
			CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="$${RUSTFLAGS:-} $${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128" \
			cargo test --locked --target wasm32-unknown-unknown -p purrdf-deflate --test deflate_conformance -- --exact \
				selected_backend_is_reported \
				every_kernel_path_encodes_and_decodes_the_same_bytes \
				copy_kernels_match_portable \
				match_length_kernels_match_portable \
				hash_kernels_match_portable; \
	fi

wasm-pkg: ## Build the purrdf npm/ESM package (release wasm + wasm-bindgen web bindings) into crates/rdf-wasm/js/pkg/.
	@# +simd128 is a PLATFORM target feature (not a Cargo feature): it turns on
	@# the wasm SIMD instruction set so memchr's byte scan (the parser hot path)
	@# runs vectorized instead of SWAR. Native BLAKE3 also selects its four-lane
	@# SIMD128 kernel from this target feature, with no semantic Cargo feature.
	@# `make simd-asm` measures this build and the portable baseline. It is
	@# scoped to this npm-artifact build only, so `make wasm` stays baseline-clean.
	@# This raises the artifact's browser baseline to engines with wasm SIMD
	@# (all major browsers since ~2021; Node >= 18, the package's engine floor).
	@# Preserve explicit environment RUSTFLAGS and deny warnings in the artifact
	@# build. Setting RUSTFLAGS replaces Cargo's configured rustflags, so those
	@# configuration values are not implicitly inherited by this command.
	@# --remap-path-prefix makes the artifact independent of WHERE it was built.
	@# rustc embeds absolute source paths (panic locations, debug info); this
	@# artifact carried 116 of them, all under the builder's home directory, so its
	@# byte size depended on the operator's USERNAME. Both varying roots are
	@# remapped onto fixed tokens. CARGO_HOME may be relocated, so its default is
	@# only a fallback.
	@# wasm-bindgen-cli must match the crate's exact wasm-bindgen pin (see [workspace.dependencies]).
	@# Shared target paths can be published by another Cargo flag variant after a
	@# build returns. Capture Cargo's registered link output from a new private
	@# target directory and bindgen that exact file; never discover it in the shared target.
	@capture=$$(mktemp -d); \
		RUSTFLAGS="$${RUSTFLAGS} -D warnings -C target-feature=+simd128 --remap-path-prefix=$(CURDIR)=/purrdf --remap-path-prefix=$${CARGO_HOME:-$$HOME/.cargo}=/cargo" \
		python3 scripts/build-private-wasm.py "$$capture/purrdf_wasm.wasm" purrdf_wasm \
			-p purrdf-wasm --lib --crate-type cdylib --target wasm32-unknown-unknown --release --locked && \
		PATH="$$HOME/.cargo/bin:$$PATH" wasm-bindgen "$$capture/purrdf_wasm.wasm" \
			--out-dir crates/rdf-wasm/js/pkg --target web
	@# The asynchronous lane's suspending import comes from ./purrdf_jspi.mjs, which the
	@# glue imports by relative path and wires into the instance's import object as is.
	@# The module ships next to the glue; a glue that does not import it would leave
	@# the raw import unresolved at instantiation, so its absence is a build failure.
	cp crates/rdf-wasm/js/src/purrdf_jspi.mjs crates/rdf-wasm/js/pkg/purrdf_jspi.mjs
	@grep -qE '^import \* as [A-Za-z_$$][A-Za-z0-9_$$]* from "\./purrdf_jspi\.mjs"$$' crates/rdf-wasm/js/pkg/purrdf_wasm.js || { \
		echo "ERROR: the wasm-bindgen glue does not import ./purrdf_jspi.mjs (expected: import * as <name> from \"./purrdf_jspi.mjs\")"; exit 1; }
	@# wasm-opt -Oz is a REQUIRED build step (roughly halves the artifact).
	@# The --enable flags cover the post-MVP features rustc emits by default
	@# for wasm32-unknown-unknown; older binaryen builds (e.g. Ubuntu's apt
	@# package) reject the module without them. --enable-simd is REQUIRED for the
	@# +simd128 build above (binaryen rejects the SIMD-carrying module without it).
	@command -v wasm-opt >/dev/null 2>&1 || { echo "ERROR: wasm-opt (binaryen) not found — it is a REQUIRED wasm build dependency:"; echo "  install binaryen version $(BINARYEN_VERSION)"; exit 1; }
	@# Pin binaryen so the optimized artifact is byte-reproducible.
	@FOUND=$$(wasm-opt --version | grep -oE '[0-9]+' | head -1); \
		test "$$FOUND" = "$(BINARYEN_VERSION)" || { \
			echo "ERROR: binaryen (wasm-opt) version mismatch — found $$FOUND, expected $(BINARYEN_VERSION)."; \
			echo "  install binaryen $(BINARYEN_VERSION) so the optimized artifact is reproducible."; \
			exit 1; \
		}
	wasm-opt -Oz \
		--enable-bulk-memory --enable-nontrapping-float-to-int \
		--enable-sign-ext --enable-mutable-globals --enable-simd \
		-o crates/rdf-wasm/js/pkg/purrdf_wasm_bg.wasm crates/rdf-wasm/js/pkg/purrdf_wasm_bg.wasm
	@# The post-link step, last because it must see the module wasm-opt ships. It
	@# exports the shadow-stack pointer, routes every call of the suspending import
	@# through an injected $suspend that puts the resumed job's own pointer back
	@# before anything else runs, wraps purrdf_jspi_run so a run starts on the region
	@# top its caller passes and returns with the idle pointer restored, and puts
	@# every exported function behind the poison gate (crates/wasm-link). It validates
	@# the module it writes and refuses one it does not recognize or has already linked.
	cargo run -p wasm-link --release --locked -- crates/rdf-wasm/js/pkg/purrdf_wasm_bg.wasm
	@# Durable proof that +simd128 actually produced SIMD codegen: a green
	@# wasm-pkg-test round-trip only proves the module runs correctly, not that
	@# it is vectorized — a memchr/RUSTFLAGS/dependency regression could ship a
	@# silently scalar artifact with every test still passing. Disassemble the
	@# optimized module and hard-fail if no SIMD opcodes are present.
	@command -v wasm-dis >/dev/null 2>&1 || { echo "ERROR: wasm-dis (binaryen) not found — it is a REQUIRED wasm build dependency"; exit 1; }
	@count=$$(wasm-dis crates/rdf-wasm/js/pkg/purrdf_wasm_bg.wasm | grep -cE 'v128|i8x16|i16x8|i32x4|i64x2|f32x4|f64x2' || true); \
		[ "$$count" -gt 0 ] || { echo "ERROR: wasm-pkg produced NO SIMD opcodes (+simd128 regressed — refusing to ship a scalar artifact)"; exit 1; }; \
		echo "OK: verified $$count SIMD opcode(s) present in the optimized wasm artifact"
	@echo "OK: purrdf npm package built (crates/rdf-wasm/js/pkg/)"

wasm-pkg-test: wasm-pkg ## Build and test the optimized npm/wasm package.
	cd crates/rdf-wasm/js && npm ci --ignore-scripts --no-audit --no-fund && npm run check
	./scripts/check-wasm-dataset-identity.sh

wasm-pkg-bench: wasm-pkg ## Build the wasm package and run the Node parse-throughput and poison-gate benchmarks (report-only; never a gate).
	cd crates/rdf-wasm/js && node bench/parse.bench.mjs && node bench/gate.bench.mjs

# The static RDF-1.2 console (docs/playground/) assembled next to a fresh copy of the
# published ESM package — the exact tree the Pages deploy ships at /playground. The app
# is zero-dependency vanilla ESM; "assembly" is just a copy, no bundler.
PLAYGROUND_OUT := $(CARGO_TARGET_DIR)/playground
playground: ## Assemble the standalone RDF-1.2 console into $(CARGO_TARGET_DIR)/playground (serve it to preview).
	@# Ship exactly the app shell + a FRESH copy of the published package. The
	@# smoke/ Node tests and any local docs/playground/purrdf/ preview copy are
	@# deliberately NOT shipped — the package is (re)built here from source.
	@# Bind runtime notices to this build, including a cached registered compiler
	@# output. Capture before the sequential build and refuse a changed compiler.
	@compiler=$$(mktemp); after=$$(mktemp); \
		trap 'rm -f "$$compiler" "$$after"' EXIT; \
		python3 scripts/package-licenses.py --compiler-record "$$compiler" && \
		$(MAKE) wasm-pkg && \
		python3 scripts/package-licenses.py --compiler-record "$$after" && \
		cmp "$$after" "$$compiler" && \
		rm -rf "$(PLAYGROUND_OUT)" && \
		mkdir -p "$(PLAYGROUND_OUT)/purrdf" && \
		cp "$$compiler" "$(PLAYGROUND_OUT)/purrdf/build-compiler.txt"
	python3 scripts/fetch-locked-deps.py
	python3 scripts/package-licenses.py --profile npm --check
	@cp docs/playground/index.html docs/playground/app.mjs docs/playground/engine.worker.mjs \
		docs/playground/sarif.mjs docs/playground/style.css docs/playground/sw.mjs \
		docs/playground/manifest.webmanifest \
		"$(PLAYGROUND_OUT)/"
	@cp -R docs/playground/examples "$(PLAYGROUND_OUT)/examples"
	@cp crates/rdf-wasm/js/index.mjs crates/rdf-wasm/js/package.json "$(PLAYGROUND_OUT)/purrdf/"
	@cp -R crates/rdf-wasm/js/pkg "$(PLAYGROUND_OUT)/purrdf/pkg"
	@cp -R crates/rdf-wasm/js/licenses "$(PLAYGROUND_OUT)/purrdf/licenses"
	python3 scripts/package-licenses.py --runtime-dir "$(PLAYGROUND_OUT)/purrdf/licenses/runtime"
	python3 scripts/package-licenses.py --profile npm --audit-directory "$(PLAYGROUND_OUT)" \
		--recipient-root purrdf --receipt "$(CARGO_TARGET_DIR)/license-evidence-playground.json"
	@echo "OK: console assembled at $(PLAYGROUND_OUT)"
	@echo "    preview: (cd $(PLAYGROUND_OUT) && python3 -m http.server 8080) then open http://localhost:8080/"

playground-smoke: playground ## Smoke the assembled console's worker dispatch and every pane's package calls (Node-side; a CI gate).
	PLAYGROUND_OUT="$(abspath $(PLAYGROUND_OUT))" node --test docs/playground/smoke/*.test.mjs

capi-build: ## Build libpurrdf (cdylib + staticlib + header + pkg-config) via cargo-c.
	cargo capi build -p purrdf-capi

capi-header: ## Regenerate the committed purrdf.h ABI contract from the crate.
	python3 scripts/capi-header.py --write $(CAPI_HEADER)

capi-check: ## Verify the committed purrdf.h is current + the C smoke links and runs.
	python3 scripts/capi-header.py --self-test
	python3 scripts/capi-header.py --check $(CAPI_HEADER)
	cargo test -p purrdf-capi --test c_smoke --locked

capi-install: ## Install libpurrdf + purrdf.pc + header to PREFIX (default /usr/local).
	python3 scripts/package-licenses.py --profile c --check
	cargo capi install -p purrdf-capi --prefix="$(if $(PREFIX),$(PREFIX),/usr/local)"
	install -d "$(if $(PREFIX),$(PREFIX),/usr/local)/share/purrdf/licenses"
	cp -R crates/rdf-capi/licenses/. "$(if $(PREFIX),$(PREFIX),/usr/local)/share/purrdf/licenses/"

.PHONY: capi-bundle license-bundles python-release-check
python-release-check: ## Build, audit and jointly install both exact Python distributions.
	bash scripts/check-python-release-artifacts.sh

license-bundles: ## Regenerate recipient license texts, notice inventories and dependency evidence.
	python3 scripts/fetch-locked-deps.py
	python3 scripts/package-licenses.py --write

capi-bundle: ## Build and audit the native C distribution, including all recipient notices.
	python3 scripts/build-capi-bundle.py
