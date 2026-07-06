# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

CARGO_TARGET_DIR ?= target
CAPI_HEADER := crates/rdf-capi/include/purrdf.h

.PHONY: help metadata fmt check book check-issue-refs changelog bump release-tags test doc bench bench-python pytest conformance rdf-core-hygiene wasm wasm-pkg wasm-pkg-test wasm-pkg-bench \
	capi-build capi-header capi-check capi-install

# The changelog generator is pinned so the committed CHANGELOG.md and the notes
# the release workflow slices out of it stay byte-reproducible across machines.
GIT_CLIFF_VERSION := 2.13.1

help: ## Show this help.
	@grep -E '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  %-18s %s\n", $$1, $$2}'

metadata: ## Regenerate + verify workspace metadata and generated artifacts.
	cargo metadata --no-deps
	bash scripts/check-generated.sh

fmt: ## Auto-format the workspace.
	cargo fmt --all

check: ## The full local gate: fmt, clippy, build, tests, hygiene.
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --locked -- -D warnings
	cargo check --workspace --lib --tests --locked
	python3 scripts/check-no-features.py
	python3 scripts/check-licenses.py
	python3 scripts/check-corpus-frozen.py
	bash scripts/check-generated.sh
	python3 scripts/check-issue-refs.py
	python3 scripts/check-versions.py
	cargo test --workspace --locked
	$(MAKE) rdf-core-hygiene
	$(MAKE) wasm

check-issue-refs: ## Reject #NNN issue-reference tokens in comments and docs.
	python3 scripts/check-issue-refs.py

changelog: ## Regenerate the deterministic CHANGELOG.md from conventional-commit history.
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

release-tags: ## Cut + push rust-v/py-v/npm-v tags for VERSION after coherence checks (make release-tags VERSION=x.y.z).
	@test -n "$(VERSION)" || { echo "usage: make release-tags VERSION=x.y.z"; exit 1; }
	@test -z "$$(git status --porcelain)" || { echo "ERROR: working tree is dirty — commit the release bump + changelog first"; exit 1; }
	@branch=$$(git branch --show-current); test "$$branch" = "main" || { echo "ERROR: release tags must be cut from main (currently on $$branch)"; exit 1; }
	@python3 scripts/check-versions.py
	@tree_version=$$(python3 -c "import tomllib;print(tomllib.load(open('Cargo.toml','rb'))['workspace']['package']['version'])"); \
		test "$$tree_version" = "$(VERSION)" || { echo "ERROR: VERSION=$(VERSION) does not match the tree version $$tree_version — run 'make bump VERSION=$(VERSION)' first"; exit 1; }
	@# Pre-tag guard: the CHANGELOG.md section is the release notes the cargo
	@# workflow slices out AFTER publishing — verify it exists BEFORE we push the
	@# irreversible rust-v tag. This awk slice is byte-identical to the one in
	@# .github/workflows/release-cargo.yaml so the local and CI guards never diverge.
	@notes=$$(awk -v v="$(VERSION)" ' \
		$$0 == "## [" v "]" || index($$0, "## [" v "] ") == 1 { flag = 1; next } \
		/^## \[/ { flag = 0 } \
		flag { print } \
	' CHANGELOG.md); \
		test -n "$$(printf '%s' "$$notes" | tr -d '[:space:]')" || { echo "ERROR: CHANGELOG.md has no release-notes section for [$(VERSION)] — run 'make changelog' and commit it before tagging"; exit 1; }
	git tag "rust-v$(VERSION)"
	git tag "py-v$(VERSION)"
	git tag "npm-v$(VERSION)"
	git push origin "rust-v$(VERSION)" "py-v$(VERSION)" "npm-v$(VERSION)"
	@echo "OK: pushed rust-v$(VERSION), py-v$(VERSION), npm-v$(VERSION)"

test: ## Run the workspace test suite.
	cargo test --workspace --locked

doc: ## Build docs for the 16 publishable crates with rustdoc warnings denied.
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --exclude purrdf-capi --exclude purrdf-python --exclude purrdf-sparql-conformance

book: ## Build The PurRDF Book (mdBook user guide) into docs/book/book/.
	mdbook build docs/book

bench: ## Run criterion benchmarks (report-only; never a gate).
	cargo bench -p purrdf-gts -p purrdf-core -p purrdf-rdf -p purrdf-sparql-eval -p purrdf-shapes

bench-python: ## Compare the rdflib compat shim vs. real rdflib (report-only; NOT a test gate). See docs/BENCHMARKS.md.
	cd bindings/python && uv run maturin develop && uv run python benchmarks/bench_compat.py

pytest: ## Build the native module + run the Python binding test suite (own gate, NOT part of `check`).
	cd bindings/python && uv run maturin develop && uv run pytest tests

conformance: ## Umbrella conformance matrix: native Rust W3C suites + the Python rdflib drop-in gate, one scoreboard (see docs/CONFORMANCE.md).
	python3 scripts/conformance-matrix.py

rdf-core-hygiene: ## Prove the kernel ring-fence: no oxigraph/PyO3 in purrdf-core, zero-dep leaves.
	@tree=$$(cargo tree -p purrdf-core --edges normal -f "{p}") || { echo "FAIL: cargo tree errored"; exit 1; }; \
	if echo "$$tree" | grep -Eq '(oxigraph|oxrdf|oxsdatatypes|oxiri|pyo3) v'; then \
		echo "FAIL: purrdf-core pulls an oxigraph-family or PyO3 crate as a NORMAL dependency"; \
		echo "$$tree" | grep -E '(oxigraph|oxrdf|oxsdatatypes|oxiri|pyo3) v'; exit 1; \
	fi; \
	echo "OK: purrdf-core has no oxigraph/PyO3 normal dependency"
	@for leaf in purrdf-iri purrdf-xsd purrdf-events; do \
		deps=$$(cargo tree -p $$leaf --edges normal --depth 1 -f "{p}" | tail -n +2); \
		if [ -n "$$deps" ]; then \
			echo "FAIL: $$leaf must stay zero-dependency but depends on:"; echo "$$deps"; exit 1; \
		fi; \
		echo "OK: $$leaf is zero-dependency"; \
	done

wasm: ## Prove the release crates build for wasm32-unknown-unknown (SKIP locally if target absent; CI hard-fails).
	@if rustup target list --installed 2>/dev/null | grep -qx wasm32-unknown-unknown; then \
		cargo check --locked --target wasm32-unknown-unknown --lib \
			-p purrdf-events -p purrdf-iri -p purrdf-xsd -p purrdf-gts -p purrdf-core \
			-p purrdf-sparql-algebra -p purrdf-sparql-results -p purrdf-sparql-eval \
			-p purrdf-rdf -p purrdf-slice -p purrdf-shapes -p purrdf-shex -p purrdf-entail \
			-p purrdf-validate -p purrdf -p purrdf-wasm; \
	elif [ -n "$${CI:-}" ]; then \
		echo "FAIL: wasm32-unknown-unknown target absent in CI"; exit 1; \
	else \
		echo "SKIP: wasm32-unknown-unknown target not installed — 'rustup target add wasm32-unknown-unknown' to enable"; \
	fi

wasm-pkg: ## Build the purrdf npm/ESM package (release wasm + wasm-bindgen web bindings) into crates/rdf-wasm/js/pkg/.
	@# +simd128 is a PLATFORM target feature (not a Cargo feature): it turns on
	@# the wasm SIMD instruction set so memchr's byte scan (the parser hot path)
	@# and blake3's simd128 backend run vectorized instead of scalar/SWAR. It is
	@# scoped to this npm-artifact build only, so `make wasm` stays baseline-clean.
	@# This raises the artifact's browser baseline to engines with wasm SIMD
	@# (all major browsers since ~2021; Node >= 18, the package's engine floor).
	@# Append rather than overwrite so any env / .cargo/config.toml RUSTFLAGS
	@# (sccache, linker args, extra target features) survive alongside +simd128.
	RUSTFLAGS="$${RUSTFLAGS} -C target-feature=+simd128" \
		cargo build -p purrdf-wasm --target wasm32-unknown-unknown --release --locked
	@# wasm-bindgen-cli must match the crate's exact wasm-bindgen pin (see [workspace.dependencies]).
	PATH="$$HOME/.cargo/bin:$$PATH" wasm-bindgen \
		$(CARGO_TARGET_DIR)/wasm32-unknown-unknown/release/purrdf_wasm.wasm \
		--out-dir crates/rdf-wasm/js/pkg --target web
	@# wasm-opt -Oz is a REQUIRED build step (roughly halves the artifact).
	@# The --enable flags cover the post-MVP features rustc emits by default
	@# for wasm32-unknown-unknown; older binaryen builds (e.g. Ubuntu's apt
	@# package) reject the module without them. --enable-simd is REQUIRED for the
	@# +simd128 build above (binaryen rejects the SIMD-carrying module without it).
	@command -v wasm-opt >/dev/null 2>&1 || { echo "ERROR: wasm-opt (binaryen) not found — it is a REQUIRED wasm build dependency"; exit 1; }
	wasm-opt -Oz \
		--enable-bulk-memory --enable-nontrapping-float-to-int \
		--enable-sign-ext --enable-mutable-globals --enable-simd \
		-o crates/rdf-wasm/js/pkg/purrdf_wasm_bg.wasm crates/rdf-wasm/js/pkg/purrdf_wasm_bg.wasm
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

wasm-pkg-test: wasm-pkg ## Build the wasm package and run the Node real-execution round-trip suite.
	cd crates/rdf-wasm/js && node --test tests/*.test.mjs

wasm-pkg-bench: wasm-pkg ## Build the wasm package and run the Node parse-throughput benchmark (report-only; never a gate).
	cd crates/rdf-wasm/js && node bench/parse.bench.mjs

capi-build: ## Build libpurrdf (cdylib + staticlib + header + pkg-config) via cargo-c.
	cargo capi build -p purrdf-capi

capi-header: ## Regenerate the committed purrdf.h ABI contract from the crate.
	@touch crates/rdf-capi/src/lib.rs  # cargo-c only re-runs cbindgen when the crate recompiles
	cargo capi build -p purrdf-capi
	@hdr=$$(find $(CARGO_TARGET_DIR) -path '*/include/purrdf/purrdf.h' | head -1); \
	  test -n "$$hdr" || { echo "FAIL: cargo-c did not emit purrdf.h"; exit 1; }; \
	  cp "$$hdr" $(CAPI_HEADER); echo "regenerated $(CAPI_HEADER)"

capi-check: ## Verify the committed purrdf.h is current + the C smoke links and runs.
	@touch crates/rdf-capi/src/lib.rs  # force cbindgen to re-run so a cached build cannot serve a stale header
	cargo capi build -p purrdf-capi
	@hdr=$$(find $(CARGO_TARGET_DIR) -path '*/include/purrdf/purrdf.h' | head -1); \
	  test -n "$$hdr" || { echo "FAIL: cargo-c did not emit purrdf.h"; exit 1; }; \
	  if ! diff -q "$$hdr" $(CAPI_HEADER) >/dev/null; then \
	    echo "FAIL: $(CAPI_HEADER) is STALE — run 'make capi-header' and commit the ABI header"; \
	    diff $(CAPI_HEADER) "$$hdr" | head -40; exit 1; \
	  fi; \
	  echo "OK: committed purrdf.h matches the libpurrdf ABI surface"
	cargo test -p purrdf-capi --test c_smoke --locked

capi-install: ## Install libpurrdf + purrdf.pc + header to PREFIX (default /usr/local).
	cargo capi install -p purrdf-capi --prefix="$(if $(PREFIX),$(PREFIX),/usr/local)"
