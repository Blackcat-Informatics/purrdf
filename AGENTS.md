<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# AI Developer Agent Guide (AGENTS.md)

Welcome, AI Agent! This file is your behavioral contract and instruction manual for
contributing to the PurRDF repository.

---

## Deficiency emergency ledger (non-negotiable)

`.deficiencies` is the log of last resort for critically undone work. Every entry
below its marker is **100% unauthorized**, is **100% a bug**, and means its
originating issue or pull request failed. It is used for work misrepresented to
pass PR gates, work misrepresented by an agent, or a discovery that an agent was
fundamentally defective. An entry is literally a cry for help from a failing
agent; it is never an accepted risk, authorized descope, backlog, or success with
caveats.

The only normal contents are the tracked notice and marker, with no entries below
them. An entry blocks completion, PR creation, and merge of the work that produced
it. Immediately verify the defect against current code and give it a durable,
visible remediation owner before removing the emergency entry. Removing an entry
does not resolve the bug or retroactively make the failed work successful. Never
add ledger text to make incomplete work appear complete.

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
| `purrdf-columnar` (`crates/columnar`) | Bidirectional five-table Parquet codec for RDF 1.2 + blobs |
| `purrdf-gts` (`crates/gts`) | GTS container engine (CBOR log, BLAKE3, COSE) |
| `purrdf-sparql-{algebra,eval,results}` | SPARQL 1.1/1.2 parser, evaluator, results |
| `purrdf-shapes` (`crates/shapes`) | SHACL validation (full Core + SHACL-SPARQL + SHACL-AF) and SHACL Rules (`sh:rule` inference) |
| `purrdf-shex` (`crates/shex`) | ShEx 2.1 schemas + validation |
| `purrdf-slice` (`crates/slice`) | Slice catalog, artifacts, ownership analysis |
| `purrdf-datalog` (`crates/datalog`) | Deterministic semi-naive Datalog substrate beneath every rule-driven engine |
| `purrdf-entail` (`crates/entail`) | Entailment regimes: RDF/RDFS/OWL 2 RL/D materialization, OWL-Direct, RIF |
| `purrdf-geo` (`crates/geo`) | GeoSPARQL 1.1: exact float-free WKT/GeoJSON geometry and the `geof:` family over both extension seams |
| `purrdf-text` (`crates/text`) | Deterministic full-text search over literals: exact fixed-point BM25, ranked rows through the property-function seam |
| `purrdf-retrieval` (`crates/retrieval`) | Composition layer over the ranked producers: plan → compile → execute → fuse, with a canonical BLAKE3 plan identity and an exact, content-addressed fusion law; producers, strata and weights are caller-supplied |
| `purrdf-validate` (`crates/validate`) | Shared string boundary every language binding routes through |
| `purrdf-json` (`crates/json`) | Ordered JSON byte-cover codec with queryable occurrences, strict reconstruction and caller-selected profile; runtime dependencies are `purrdf-core`, `purrdf-lex` (the JSON string decoder and pointer tokens) and the `purrdf-hash` root |
| `purrdf-hash` (`crates/hash`) | Native, zero-dependency hashing: BLAKE3-256, MD5 (RFC 1321), SHA-1 (FIPS 180-4), SHA3-224/256/384/512 (FIPS 202) and CRC-32 (ISO-HDLC), streaming and one-shot; SHA-1 and CRC-32 run on the x86 SHA/`pclmulqdq` and Armv8 SHA1/CRC32 instructions when detected at run time, the portable source otherwise; the SPARQL hash built-ins, OpenPGP fingerprints and derivation identities compute through it. Also `fixed::FixedHasher`, the workspace's fixed-key table hasher: folded multiplies, or an AES accumulator on a build whose target enables AES (compile-time only, never run-time detection; wasm32 and 32-bit targets use the portable function), each function pinned by frozen self-vectors. The workspace root: zero dependencies, and every crate may depend on it; it also holds the small specified kernels shared across the workspace (`fnv`, FNV-1a 64-bit; `mix`, SplitMix64) and the `Backend` trait every family of named execution paths implements (`selected`, `is_available`, `all_available`, `name`), with the one `PURRDF_REQUIRE_SIMD_PATHS` check in `dispatch` |
| `purrdf-deflate` (`crates/deflate`) | Native DEFLATE (RFC 1951) and gzip (RFC 1952): a push-based streaming decoder that decodes every gzip member, verifies each trailer and refuses trailing garbage and output past a caller's limit, and a deterministic encoder (gzip `MTIME` 0, `XFL` 0, `OS` 255; the same bytes on every path and however the input is chunked); match copies, match-length compares and window hashing run on SSE2/AVX2, NEON or wasm simd128, portable code otherwise; sole runtime dependency is `purrdf-hash` (the CRC-32) |
| `purrdf-jsonschema` (`crates/jsonschema`) | Native JSON Schema validation for drafts 2020-12, 2019-09 and 07, each schema resource in its own dialect: every vocabulary, `$dynamicRef`, `$recursiveRef`, `unevaluated*`, `$vocabulary`, the flag/basic/detailed output formats, exact decimal numbers, and ECMA-262 `/u` patterns using `regex` for regular expressions and a bounded explicit-stack matcher for lookaround, backreferences and scoped modifiers; `Schema::is_valid`/`evaluate` return typed errors on resource exhaustion; depends on `serde_json`, `regex`, `purrdf-iri` and `purrdf-hash` only |
| `purrdf-markdown` (`crates/markdown`) | Structural Markdown-to-RDF 1.2 slicer under a shipped specification: a typed stand-off model over verbatim byte spans, projected to claims; sole runtime dependency is `purrdf-core` |
| `purrdf-hash` | The zero-dependency root |
| `purrdf-lex` (`crates/lex`) | Native lexical foundations shared by every grammar, over the zero-dependency `purrdf-hash` root: the exact Turtle/SPARQL/XML terminal classes and escape decoders (`terminals`: `decode_uchar`, `echar_value`, `decode_char_ref`, `skip_ws`/`trim_ws`, `is_ncname`, `in_ranges`), the chunked byte-class scanners and `ByteClass` kernel that lower to packed compares on SSE2/AVX2/AVX-512, NEON and wasm simd128, with `find_byte`/`find_byte2` for needles known at run time (`scan`), the RFC 8259 JSON string escaper and decoder every JSON writer and reader shares (`json_escape`), RFC 6901 JSON Pointer tokens (`json_pointer`) and RFC 3986 percent-encoding (`percent`); its scope is the workspace's lexical layer — byte-class scanning, terminals, term syntax, literal/IRI escaping, percent encoding, JSON strings and pointers, a JSON reader/writer, an XML reader and Unicode normalisation |
| `purrdf-iri` (`crates/iri`) | IRI/URI value space (RFC 3987/3986 parse, resolution, normalization, CURIEs, BCP 47 tags, IDNA2008); runtime dependencies are `purrdf-lex`, whose `terminals`, `scan`, `json_escape`, `json_pointer` and `percent` it re-exports, and the `purrdf-hash` root |
| `purrdf-xsd` | Foundation over `purrdf-hash` alone |
| `purrdf-events` | Zero-dependency foundation |
| `purrdf-deflate` | Leaf over `purrdf-hash` alone |
| `purrdf-cdt` (`crates/cdt`) | SPARQL composite datatypes (SEP-0009 `cdt:List`/`cdt:Map`): closed leaf over `purrdf-iri` + `purrdf-xsd` only |
| `purrdf-stack` (`crates/stack`) | How much stack the thread has left (native OS limit, read via target-gated `libc`/`windows-sys` declarations with no C toolchain needed; wasm32 shadow stack against an installable floor) and the margin the SPARQL evaluator refuses at |
| `purrdf-wasm`, `purrdf-capi`, `bindings/python` | WASM, C-ABI, and PyO3 bindings |
| `purrdf-cli` (`crates/cli`) | The `purrdf` command-line surface (`publish = false`) |
| `purrdf-envelope-probe` (`crates/envelope-probe`) | The micro-hardware envelope capture tool (`publish = false`) |
| `purrdf-alloc-probe` (`crates/alloc-probe`) | The shared counting allocator + per-thread/whole-process measurement windows every allocation test and bench measures with (`publish = false`, `[dev-dependencies]` only, path-only with no `version`) |
| `purrdf-bench` (`crates/bench`) | Benchmark tooling: the scale-corpus generator (`publish = false`) |
| `purrdf-testkit` (`crates/testkit`) | Shared test support: byte-exact goldens (`assert_golden!`), temporary paths under the target directory (`temp_dir!`, `temp_file!`, `for_unit_test`), self-hashing frozen differential vectors, the libtest-compatible `harness = false` runner, the property harness (`prop_test!`: choice-sequence shrinking, regex string generators, stateful model testing, a deterministic seed per property), and the micro-benchmark harness every bench target runs on (`purrdf_testkit::bench`, `bench_group!`/`bench_main!`: warm-up, flat sampling, median with MAD and a seeded bootstrap interval, throughput, saved baselines compared with a bootstrapped change, a fixed-schema `estimates.json` per benchmark, natively and on wasm32); its one first-party dependency is `purrdf-hash`, the root, whose own tests do not use testkit, so no member's tests close a cycle through it (`publish = false`, `[dev-dependencies]` only, path-only with no `version`) |
| `wasm-link` (`crates/wasm-link`) | The wasm package's post-link step: links the suspend, run and poison guarantees into the optimized module (`publish = false`, host tool) |
| `purrdf-hash-conformance` (`crates/hash-conformance`) | The frozen-vector suites of `purrdf-hash` (digest differentials, BLAKE3 streaming boundaries, base16 rendering, the table hasher's self-vectors and quality), the digest, base16 and BLAKE3 throughput bench and the table hasher's latency bench, on testkit's runner and bench harness, natively and on wasm32; separate from `purrdf-hash` because testkit depends on it (`publish = false`) |
| `helper-census` (`crates/helper-census`) | The structural helper census: normalises every shipping function body (local names renamed, literals abstracted) and reports isomorphic bodies, repeated thin forwarders, constants by value and hex-digit tables against `helpers-ledger.toml` (`publish = false`, host tool) |

## 2. Hard constraints (violating these fails CI or review)

* **NO semantic Cargo features, ever.** The sole exception is the empty,
  non-semantic `purrdf-capi:capi = []` marker that `cargo-c` requires. It gates
  no code and must never appear in `cfg(feature = ...)`; CI checks both facts
  with `scripts/check-no-features.py`. PurRDF is a carrier; optionality changes
  semantics per consumer, which is forbidden. Do not add any other feature,
  optional dependency, or feature-gated behavior.
* **Kernel ring-fence.** `purrdf-core` must never depend on oxigraph or PyO3.
  `purrdf-hash` has **zero runtime dependencies**; `purrdf-lex`, `purrdf-iri`,
  `purrdf-xsd`, `purrdf-events`, `purrdf-testkit` and `purrdf-deflate` depend
  only on crates listed in their `layers.toml` rows (`make rdf-core-hygiene`
  checks both, reading the ring-fenced crates and their rows from
  `layers.toml`).
* **One home per job.** `helpers-ledger.toml` names the single implementation
  of each job the workspace provides once, what it replaces, and each sanctioned
  second implementation with its criterion and documented reason;
  `scripts/check-shared-helpers.py` (in `make check`, or `make helpers-hygiene`)
  runs `crates/helper-census` and fails on an unresolvable home, a forbidden copy
  outside an enforced job's home, a stale exemption, or a `#[path]` include that
  leaves its crate. `layers.toml` declares which first-party crate may depend on
  which, with `purrdf-hash` the root every crate may use;
  `scripts/check-layers.py` (in `make check`, or `make layer-hygiene`) fails on
  any first-party normal edge it does not allow and on any row the resolved graph
  no longer matches, and `--home-for A B …` names the common dependency
  closest to the given callers — where one implementation they share belongs.
* **Terminal ring-fence: a scanner's character classes are exact, in both
  directions.** They decide **token boundaries**, not merely membership, so
  substituting a Unicode property for a production's enumerated set does not
  just widen the accepted language — it silently *re-tokenizes documents both
  the liberal and the conforming parser accept*. `?s<NBSP>?p` lexed as one
  variable, turning a join into a cross product with exit zero and no
  diagnostic. Every W3C terminal is spelled **once**, in
  `purrdf_lex::terminals` (re-exported as `purrdf_iri::terminals`), with its production cited and its ranges asserted at
  compile time; scanners call it rather than retyping a table.
  `scripts/check-terminal-predicates.py` (in `make check`, or
  `make terminal-hygiene`) refuses a Unicode-property test inside a file that
  holds a character cursor, and names in `SCANNERS` the files whose cursor lives
  in another module. This is **not** "Unicode properties are bad": a production
  that *names* one must be implemented with it, and the gate's `ALLOWLIST` is
  the reasoned ledger for exactly those cases. **Judge the clause, not the
  specification** — one spec answers this differently in different places.
  CommonMark defines a "Unicode whitespace character" and uses it for §6.2
  emphasis flanking, while its blank line (§2.1), ATX heading, thematic break
  and GFM table cell all name space-or-tab; citing "CommonMark" alone settles
  nothing, and doing so once put a false exemption into this file.
* **Everything is wasm-able.** Every release crate (all 30 publishable crates,
  `purrdf-wasm` included) must build for `wasm32-unknown-unknown` — CI
  hard-fails otherwise (`make wasm` locally). Never add a dependency that
  drags in threads, the filesystem, C toolchains, or wall-clock/RNG syscalls
  on the wasm path; crypto stays pure-Rust for exactly this reason.
* **Byte determinism.** Serializers and the GTS writer are byte-deterministic.
  If your change alters emitted bytes, you must update the affected goldens and
  say why in the PR. Never introduce iteration-order, time, or RNG dependence
  into output paths (hashers are the fixed-key `purrdf_hash::fixed::FixedHasher`
  for this reason). No `std` `HashMap`/`HashSet` is left on its random default hasher
  anywhere, tests and benches included: clippy bans the `RandomState` types and
  constructors, and the `fixed-hasher-everywhere` job in `helpers-ledger.toml`
  refuses every other spelling (`HashMap<K, V>`, `HashMap::default()`, `from`,
  `collect`) — name `purrdf_core::FastMap`/`FastSet` or a `FixedState` map.
* **Conformance corpora are the contract**: W3C SPARQL 1.1
  (`crates/sparql-conformance`), the W3C SHACL suite (`vectors/shacl/`), the
  shexTest v2.1.0 suite (`vectors/shexTest/`), the first-party SHACL corpus
  (`crates/shapes/corpus/`), RDFC-1.0 fixtures
  (`crates/rdf/tests/fixtures/rdfc/`), the official JSON-Schema-Test-Suite
  (`crates/jsonschema/tests/suite/`), and the **frozen** GTS vectors in
  `vectors/` (shared byte-exact with the other GTS engines — never regenerate or
  "fix" them here; the GTS wire format is governed in `gmeow-gts`). Harnesses
  assert exact counts and enforce XPASS discipline on their xfail ledgers —
  see [`docs/CONFORMANCE.md`](./docs/CONFORMANCE.md) for the scoreboard.
* **PurRDF is NOT an ontology.** Structural Markdown and ordered JSON codecs
  offer explicitly named standard profiles and vocabularies under their shipped
  specifications; callers must select them deliberately or supply a vocabulary.
  There is no implicit namespace fallback. Every other
  vocabulary the library reads or writes (slice manifests, statement-metadata
  downcast, box roles, language retagging, SPARQL extension-function
  namespaces, standpoint predicates, json_schema namespaces) is
  **caller-supplied configuration with no fabricated default**: a feature
  exercised without its vocabulary hard-errors or stays inactive. Never
  hardcode a `blackcatinformatics.ca` namespace in library code (the GMEOW
  ontology is a *consumer*; the dependency arrow never points from purrdf to
  it). Test fixtures use `example.org`.
* **Generated artifacts** under `generated/` are projections — never hand-edit;
  regenerate via `make metadata` (`scripts/check-generated.sh` gates drift).
* **Dependency versions live in one place**: `[workspace.dependencies]` in the
  root `Cargo.toml`. Member crates use `dep.workspace = true`. Do not pin a
  version inside a member manifest.
* **Lints are workspace-inherited** (`[workspace.lints]`, clippy pedantic +
  nursery). `cargo clippy --workspace --all-targets` must be warning-free.
  Prefer fixing code over `#[allow]`; a genuinely-right allow must be tightly
  scoped and carry a reason comment.
* **SPDX headers** on every source file: `MIT OR Apache-2.0 OR MulanPSL-2.0` (docs may be
  `CC-BY-4.0`).

## 3. Commands

```bash
make check      # the full local gate: fmt, clippy, build, tests, hygiene
make test       # cargo test --workspace
make metadata   # regenerate + verify generated artifacts
make bench      # purrdf_testkit::bench benchmarks (report-only; not a gate)
make scale-corpus  # generate the deterministic scale corpus (streams; stores nothing by default)
make lubm       # the LUBM comparison workload, per entailment regime (report-only; network + JRE)
make watdiv     # the WatDiv comparison workload over a frozen dataset (report-only; network)
make build-profile-hygiene  # prove the gate is compiled the way it claims
```

`scale-corpus`, `lubm` and `watdiv` are the three comparison lanes. None is a
gate and none runs in `make check`: `lubm` needs a JRE and fetches a
GPL-2.0-or-later generator, `watdiv` fetches a 58 MB frozen dataset that expands
past a gigabyte, and neither vendors a byte. They share one implementation of
the laws that make their numbers evidence — `scripts/lane-common.sh` — so a
repair to one is a repair to all three. `docs/BENCHMARKS.md` owns the
parameters, the knobs and the comparison rules.

Toolchain: `rust-toolchain.toml` names a **floating nightly** for development and
for every CI gate. That is an analysis decision, not a licence: nightly clippy
and rustdoc carry lints stable lacks, and its default borrow checker is the
stronger one, so a finding is a real finding rather than a channel artifact.
Floating is the point — a dated channel freezes that surface as of one day, and
every check sharpened afterwards stops being a finding and becomes invisible debt
while the gates still report green. Byte-determinism is no argument for freezing:
it is a property of the code — sorted, deduplicated, explicitly ordered output,
identities content-addressed over this workspace's own declared law — and the
goldens and vectors prove it on whatever compiler runs them. A golden that moved
under a compiler bump would be a serializer defect to fix, not a reason to stop
bumping.

**The source stays nightly-free.** There are zero `#![feature(...)]` attributes
in `crates/` and `bindings/`, and adding one is forbidden. What consumers need
is `rust-version` in `Cargo.toml` (the MSRV, currently 1.98) — a *lower* floor on
the *stable* channel — enforced by the dedicated `msrv` CI job. Never "align" the
MSRV to the dev pin; they answer different questions.

Two traps, both load-bearing:

* `dtolnay/rust-toolchain` selects with `rustup default`, which ranks **below**
  `rust-toolchain.toml`. A workflow that installs one toolchain while the repo
  pins another does not fail — it silently runs the pin. Only
  `RUSTUP_TOOLCHAIN` outranks the file, which is why the `msrv` job and the
  release lanes set it explicitly, and why the `msrv` job also asserts
  `rustc --version` really is 1.98.x.
* `scripts/check-toolchain-pin.py` (in `make check` and CI) fails on any
  workflow whose install step disagrees with the pin without that explicit
  escape, and on a floating channel.

Release lanes (`release-cargo`, `release-npm`, `release-pypi`) build on
**stable** on purpose: nightly's sharper lints buy nothing for an artifact a
consumer installs, and shipping one from an unreleased compiler is risk without
upside.

## 4. Performance discipline

This library is a hot-path backbone. The IR is an immutable, value-interned
dataset (`TermId` = niche-optimized `NonZeroU32`, string arena, store-once
interner). When touching parse/serialize/eval paths:

* **Measure first** — layout and algorithm choices are justified by the
  benches (`crates/rdf-core/benches/ir_layout.rs` et al., on the
  `purrdf_testkit::bench` harness), not by assertion. Add or extend a bench
  when you claim a win.
* Avoid per-token/per-term `String` allocation; move values out of buffers
  instead of cloning; pre-size collections in parse loops.
* Hot maps use the fixed-key `purrdf_hash::fixed::FixedHasher` (`FixedState`,
  or `purrdf_core::FastHasher`; see `crates/rdf-core/src/ir/builder.rs` for
  the canonical store-once interner pattern) — never default SipHash in a hot
  path, and never a randomly-seeded hasher in an output path.

**The gate is compute, so the gate is compiled like it.** `make check` runs the
whole test surface and `make conformance` runs every W3C suite through it, so
both are bounded by the codegen under them: `[profile.dev]` builds everything —
our crates, dependencies, and (named separately, because they do not inherit the
base profile) build scripts and proc-macros — at **opt-level 3**, with
`debug-assertions` and `overflow-checks` **ON**. Those two are orthogonal to
opt-level and are not negotiable: first-party code carries
`#[cfg(debug_assertions)]` bodies that vanish silently with the flag, and
overflow checks are what stop an arithmetic bug in a byte-deterministic codec
becoming a wrong-but-green run. Optimizing the gate does not weaken it: the same
assertions run over the same corpora, on better codegen.

This is not self-enforcing, and it did fail once: four per-crate `opt-level = 2`
tables made the manifest *read* as tuned while twenty-one members compiled at
opt-level 0, because `package."*"` matches dependencies only. So the property is
asserted against the graph Cargo actually resolves, not against the manifest:
`make build-profile-hygiene` (in `make check`) reads the `--unit-graph` of
`cargo test` and `cargo build` and checks every unit's **effective** profile.
Reading effective values is deliberate — it catches a `[profile.*]` table in
`$CARGO_HOME/config.toml` or anywhere on the walk up from the workspace (your
home directory is on that walk), a `CARGO_PROFILE_*` variable, or a `--config`
override, none of which the manifest can see. The `[profile.test]` block
exists and sets only `debug = 0` (test binaries carry no debug info). It must
not set `opt-level`, `debug-assertions` or `overflow-checks` — `test` inherits
those from `dev`, and `scripts/check-build-profiles.py` asserts the inherited
values — and neither it nor `[profile.dev]` may set `lto` or
`codegen-units = 1`: both serialize codegen and inflate link memory, which is
what a constantly-rebuilt, cold-in-CI gate wants least.

## 5. Brand & naming

The project name is written **PurRDF** in prose (never PurrDF/PURRDF); all
package/crate/binary identifiers are lowercase `purrdf`. See
[`docs/BRAND.md`](./docs/BRAND.md). Logo/social assets follow the shared
black-cat family system — `#cat-head-core` is shared verbatim; only the
`#service-triple` group is purrdf-specific.

## 6. Releases

Tag-driven trusted publishing: `rust-v*` → crates.io (30 crates, ordered),
`py-v*` → PyPI (`purrdf`). See [`docs/RELEASE.md`](./docs/RELEASE.md). Version
is single-sourced in `[workspace.package]`. Eleven members never reach
crates.io: `purrdf-capi`, `purrdf-sparql-conformance`,
`purrdf-hash-conformance`, `purrdf-cli`, `purrdf-envelope-probe`,
`purrdf-bench`, `purrdf-alloc-probe`, `purrdf-testkit`, `wasm-link`,
`helper-census`, and `purrdf-python` (PyPI via maturin instead).
`purrdf-alloc-probe` and `purrdf-testkit` are dev-dependencies of published
crates, so their root `[workspace.dependencies]` entries are path-only with
**no `version`** — cargo then strips them from the packaged manifest, which is
the only way `cargo publish`'s dev-dependency-resolving verification step can
succeed.

## 7. Provenance

This repo was extracted from `gmeow-ontology` and `gmeow-gts` — see
[`PROVENANCE.md`](./PROVENANCE.md) for source commits. PurRDF's replacement
surfaces are being completed here, but the downstream `gmeow-ontology` cutover
is not yet complete. Its legacy models are migration evidence only and should
be deleted as each PurRDF replacement is integrated.
