# Agent guide (AGENTS.md)

This is the one contract for every coding agent in the PurRDF repository.
`CLAUDE.md` only imports it. Where a generic guide or skill disagrees with this
file, this file wins.

**We do not care about binary size or WASM size. If you find yourself optimizing
for either, stop.**

---

## Principles

When requests conflict, these decide. They do not replace the hard constraints
below or a release plan.

* **GREENFIELD:** we always correct the mistakes of the past. Others' mistakes
  are our lessons, not our limits. No compromises inherited from earlier work.
* **SOTA:** always choose the state of the art over the bones of the past.
* **OPTIMAL:** choose the Pareto frontier of the quality, speed, efficiency and
  other axes.
* **MAXIMAL:** support the broadest functionality possible after the other
  choices are made.

Project commitments:

* **Rust first, Python surface.** All core functionality is high-quality Rust.
  Other languages are surfaces over the one engine.
* **Subsume, extend, enhance.** If something we depend on is substandard or
  holds us back, take over its job, extend it, then improve it well past the
  original.
* **Low optionality, hard failures.** The build has no optional components,
  conditional imports or optional packages, and nothing swallows an error. Fail
  early with an actionable message. This is a rule about the build and its
  packaging, not about ordinary control flow in code.
* **RDF 1.2 only.** Triple terms, reifiers and annotations are first-class
  everywhere. No shortcut that only serves RDF 1.1 and no lowering that drops
  them.
* **Three maximums:** utility (when in doubt, the more useful behavior),
  performance (the best Rust solution, using the language fully) and
  portability (code that builds for WebAssembly is the core value).

## How to work

* **Done means merged.** Work in a pull request or a worktree is not done.
  Land small, complete changes; do not grow a large change set in search of a
  perfect one, and do not iterate on a branch when one push would do. With more
  than three open pull requests or worktrees, start nothing new: land what is
  open, oldest first. Anything held more than twelve hours is reported.
* **No deferrals.** No follow-up, "left for later" or stub. The work is now.
* **You see a bug, you own it.** Fix it; do not spend the effort on who
  introduced it. A gate already failing on untouched `main` is reported, not
  silently absorbed into an unrelated change.
* **Gap analysis is adversarial.** Assume the code under review does not meet
  its specification until the evidence shows it does.
* **No process references in the repository.** Issue numbers, pull-request
  numbers and development history belong on the forge. Text such as "per the
  issue" or "will be fixed in" is banned from source, comments and
  documentation; remove it where you find it. No tool or agent credits anywhere.
* **A refusal is a claim too: prove it.** Rejecting valid input is the mirror
  of silently dropping it, and it passes every test because it looks like
  strictness. For every refusal you add or tighten, execute two cases: the one
  you believe is invalid, and a neighbouring valid one that must still
  succeed. Cost and size estimators are the worst offenders.
* **Research is encouraged; copying is not.** Read specifications, upstream
  documentation and issue trackers instead of guessing. Send only the technical
  question, never private data. Write your own code and prose; record what you
  consulted where the repository keeps its sources (`PROVENANCE.md`).
* **Look before you build.** Consult `helpers-ledger.toml`, the helper census
  and the crate map before writing something new. Prefer an existing home in
  this workspace to a second implementation.
* **Design.** Apply SOLID judged by concrete consequences. Prefer small crates
  and keep a source file under 5,000 lines; split one that grows past it.
  Refactor code your change touches when that makes the change simpler.

### Directories and data

* All git work happens in a worktree under the main checkout's `.worktrees/`.
* Notes, evidence, logs, checkpoints and scratch go under the main checkout's
  `.stage/` (ignored by git). Nothing you need is kept in `/tmp` or
  `/var/tmp`, and nothing is stored in another repository's tree.
* A worktree is removed only after every agent given it has ended, and its
  review files are saved under `.stage/` first.

### Shell commands

* One shell call does one thing. Do not join steps with `&&` or `;`. A run of
  several steps is a script file, run once, with its output in a log.
* A deletion is its own call, on one exact path inside the repository, after
  that path has been listed. Do not use `rm -r`, `find -delete`, `git clean` or
  a forced worktree removal, and do not put a deletion beside other commands.
* Run nothing that can raise a permission prompt.
* A call that was refused is not sent again in another form.

### Builds, waiting and evidence

* Cargo runs through the managed wrapper; do not work around it. Serialize
  every heavy command (cargo, a test suite, a gate, a commit whose hook
  builds) through one lock, `flock .stage/build.lock`, so that one runs at a
  time across all agents of a session.
* Do not watch. No `tail -f`, `sleep`, process listing or polling loop to
  learn that something finished. Run the command in the foreground with a
  timeout that fits it, or in the background and act on its completion notice.
  If you cannot name the event that will wake you, you are stuck: read the
  state once and act.
* Send output to a file under `.stage/` and record the tool's own exit status
  after it. A status read through a pipe, or a line you echoed yourself, is not
  evidence. A cargo command names its own tree, and its log starts with the
  directory and the commit.
* Evidence for a gate comes from the gate's own command, flag for flag. A
  hand-written command that differs from it proves nothing about the gate.
  Missing tools, skips, zero selected cases and interrupted runs are not passes.
* Read the clock when you write a time; do not estimate it.

### Sub-agents and review

* A brief is all a sub-agent knows: the goal, what is ruled out, the files to
  read, the exact scope, the worktree path and the rules that bind it. If you
  cannot write the brief, you do not understand the task well enough.
* One coding sub-agent per worktree. Read-only work (research, review, log
  reading) may run beside it.
* A reviewer gets the change and the requirement, never your conclusion. Its
  verdict counts only for the exact head it names, with what it read and what
  it did not read. Every review item is addressed before merge.
* What a sub-agent reports is a claim. Read the evidence before you repeat it
  or build on it. Judge a coder by its diff, logs and the hook.
* Before merging, the session that merges reads the merging diff itself, after
  the independent review and not in place of it. Hosted checks must be green on
  that exact head.

---

## 1. What this repository is

PurRDF is the **RDF 1.2 toolkit** that several downstream projects (notably
[`gmeow-ontology`](https://github.com/Blackcat-Informatics/gmeow-ontology)) use
as their **data-carrier backbone**. It must stay fast, deterministic, and
boring: one engine, one behavior, carried verbatim into Rust, Python,
WebAssembly, and C.

Crate map (all under `crates/` unless a path is given; each crate's README
holds the detail):

| Crate | Role |
|---|---|
| `purrdf` | Umbrella facade |
| `purrdf-core` (`crates/rdf-core`) | Interned IR kernel, store traits, diagnostics, provenance, canonicalization |
| `purrdf-rdf` (`crates/rdf`) | Text, XML and JSON-LD codecs, GTS adapters, describe |
| `purrdf-columnar` | Five-table Parquet codec for RDF 1.2 and blobs |
| `purrdf-gts` | GTS container engine: CBOR log, BLAKE3, COSE, signatures |
| `purrdf-sparql-{algebra,eval,results}` | SPARQL 1.1/1.2 parser, evaluator, results |
| `purrdf-shapes` | SHACL validation and SHACL Rules |
| `purrdf-shex` | ShEx 2.1 schemas and validation |
| `purrdf-datalog`, `purrdf-entail` | Semi-naive Datalog substrate; entailment regimes (RDF, RDFS, OWL 2 RL, D, OWL-Direct, RIF) |
| `purrdf-slice` | Slice catalog, artifacts, ownership analysis |
| `purrdf-geo`, `purrdf-geo-kernel` | Geographic functions over an exact geometry and geodesy kernel |
| `purrdf-text`, `purrdf-retrieval` | Full-text search with exact BM25; ranked retrieval plans and fusion |
| `purrdf-json`, `purrdf-toml`, `purrdf-markdown`, `purrdf-mime`, `purrdf-jsonschema`, `purrdf-glob` | Lossless document covers and validators, each under its shipped specification |
| `purrdf-validate` | The string boundary every language binding and host routes through |
| `purrdf-hash` | The zero-dependency root: digests, the fixed-key table hasher, shared small kernels, error inspection |
| `purrdf-lex` | Lexical layer: terminals, scanners, JSON, YAML, CBOR, XML, escapers, Unicode normalization |
| `purrdf-iri`, `purrdf-xsd`, `purrdf-events`, `purrdf-cdt` | IRIs and W3C vocabulary terms; XSD values and exact arithmetic; the event protocol; composite datatypes |
| `purrdf-deflate`, `purrdf-zstd`, `purrdf-ed25519` | Native DEFLATE and gzip; Zstandard; Ed25519 |
| `purrdf-stack` | Remaining stack and running a computation on a stack of a stated size |
| `purrdf-wasm` (`crates/rdf-wasm`), `purrdf-capi` (`crates/rdf-capi`), `bindings/python` | WebAssembly, C and Python bindings |
| `purrdf-cli` | The `purrdf` command |
| `purrdf-testkit`, `purrdf-alloc-probe`, `purrdf-conformance-kit` | Published development libraries: goldens, properties, benches; allocation measurement; conformance grading |
| `purrdf-sparql-conformance`, `purrdf-shacl-conformance`, `purrdf-hash-conformance` | Conformance harnesses |
| `helper-census`, `wasm-link`, `purrdf-bench`, `purrdf-envelope-probe` | Host tools: the structural census and non-Rust ratchet, the wasm post-link step, the scale corpus, the hardware envelope |

Three files are authoritative and are checked, so read them instead of a copy
here: `layers.toml` (which crate may depend on which, and the external packages
each ring-fenced crate may use), `helpers-ledger.toml` (the one home of every
shared job, its specification, vectors and sanctioned variants) and
`dependency-ledger.toml` (every external package, its reason and its verdict).
`scripts/check-banned-deps.py` lists the packages the native homes replaced and
refuses them on every edge.

## 2. Hard constraints (violating these fails CI or review)

* **Preserve the v4 compatibility baseline.** Follow
  [Compatibility after 4.0](./docs/MIGRATION-4.0.md#compatibility-after-40):
  existing signatures, variant payloads, ownership, trait requirements, host
  behavior and valid v4 artifact readers stay. Non-exhaustiveness permits
  additions, not replacement. New semantic and identity laws carry explicit
  versions. Fixed C layouts grow by additive handles and entry points.
* **No semantic Cargo features, ever.** The sole exception is the empty
  `purrdf-capi:capi = []` marker that `cargo-c` requires; it gates no code and
  never appears in `cfg(feature = ...)` (`helper-census --no-features`). No
  optional dependency and no feature-gated behavior: a carrier whose semantics
  vary per consumer is not a carrier.
* **Tooling and tests are Rust.** Checks, generators, gates and tests are
  written in Rust. Python and JavaScript remain only where a language surface
  itself is exercised. The non-Rust ratchet (`helper-census
  --non-rust-ratchet`, `crates/helper-census/src/non_rust.rs`) compares the
  target tree with the merge-base on the integration branch, by path, with no
  rename detection:
  1. **Explain.** A new path holding non-Rust code (`.py`, `.pyi`, `.js`,
     `.mjs`, `.cjs`, `.ts`, `.tsx` and their variants, a Python or JavaScript
     shebang, or such a heredoc inside a shell script) carries within its first
     40 lines a `# Why not Rust: <reason>` or `// Why not Rust: <reason>`
     comment of at least 30 characters. `vectors/`, `generated/` and files of
     whitespace are exempt.
  2. **Ratchet.** An existing path under `scripts/`, `bindings/python/tests/`,
     `crates/*/tests/`, `crates/rdf-wasm/js/tests/` or
     `crates/rdf-wasm/js/bench/` may not gain non-Rust lines. Shipped surfaces
     (the JavaScript package, the playground, the Python package) are not
     ratcheted.

  It runs in the commit hooks, in `make check` and in CI. The integration
  branch defaults to `origin/main`; `PURRDF_RATCHET_BASE` names another.
* **Kernel ring-fence.** `purrdf-core` never depends on PyO3. `purrdf-hash`
  has zero runtime dependencies. Each ring-fenced crate depends only on what
  its `layers.toml` row lists (`make rdf-core-hygiene`,
  `scripts/check-layers.py`).
* **One home per job.** Call the home `helpers-ledger.toml` names; never write
  a second body. `scripts/check-shared-helpers.py` fails on an unresolvable
  home, a forbidden copy, a stale exemption or a `#[path]` include that leaves
  its crate. `scripts/check-layers.py --home-for A B` names where an
  implementation shared by those callers belongs. Editing
  `crates/rdf-core/src/dataset_view.rs`, anything under
  `crates/rdf-core/src/ir/` or `crates/lex/src/allocation.rs` stales ledger
  identities: refresh them last.
* **Terminal ring-fence.** A scanner's character classes are exact in both
  directions, because they decide token boundaries and not only membership: a
  Unicode property in place of a production's enumerated set re-tokenizes
  documents both parsers accept. Every W3C terminal is spelled once, in
  `purrdf_lex::terminals`, with its production cited; scanners call it.
  `scripts/check-terminal-predicates.py` refuses a Unicode-property test in a
  file that holds a character cursor, and its `ALLOWLIST` records the
  productions that really name one. Judge the clause, not the specification:
  one specification answers this differently in different places.
* **Everything is wasm-able.** Every publishable crate builds for
  `wasm32-unknown-unknown` (`make wasm`). No dependency that brings threads,
  the filesystem, a C toolchain, or ambient clock or random reads onto that
  path; hosts supply those capabilities explicitly.
* **Byte determinism.** Serializers, content identities and the GTS writer are
  byte-deterministic: no iteration-order or ambient-entropy dependence in an
  output path. No `std` map or set on its random default hasher anywhere,
  tests and benches included; name `purrdf_core::FastMap`/`FastSet` or a
  `FixedState` map. A change to emitted bytes updates the affected goldens and
  says why.
* **Conformance corpora are the contract.** The W3C SPARQL, SHACL and
  canonicalization suites, shexTest, the JSON Schema suite, the first-party
  corpora and the **frozen** vectors under `vectors/` are never regenerated or
  "fixed" here; the GTS wire format is governed in `gmeow-gts`. Harnesses
  assert exact counts and police their expected-failure ledgers
  ([`docs/CONFORMANCE.md`](./docs/CONFORMANCE.md)).
* **PurRDF is not an ontology.** It mints no vocabulary. Every vocabulary the
  library reads or writes is caller-supplied configuration with no fabricated
  default: a function used without its vocabulary is a hard error or inactive.
  Codecs with a shipped specification offer named profiles the caller selects
  deliberately. Never hardcode a `blackcatinformatics.ca` namespace in library
  code. Fixtures use `example.org`. W3C Recommendation terms are the one
  built-in vocabulary, named through `purrdf_iri::vocab` and
  `purrdf_xsd::datatype`, never as string literals.
* **Generated artifacts** under `generated/` are projections: never hand-edit;
  regenerate with `make metadata`.
* **Dependency versions live in one place:** root `[workspace.dependencies]`.
  Members use `dep.workspace = true`.
* **Lints are workspace-inherited** (clippy pedantic and nursery) and must be
  warning-free. Fix the code; a justified `#[allow]` is tightly scoped and
  carries a reason.
* **SPDX headers** on every source file: `MIT OR Apache-2.0 OR MulanPSL-2.0`
  (documentation may be `CC-BY-4.0`).
* **`.deficiencies` is an emergency ledger, never an authorization.** Its
  normal content is the notice and marker only. An entry below the marker is a
  bug and a failed piece of work: it blocks completion, pull-request creation
  and merge of the work that produced it until the defect is verified and has a
  durable, visible owner. Removing an entry resolves nothing. Never add one to
  make incomplete work look complete.

## 3. Commands

```bash
make check      # the full local gate: fmt, clippy, build, tests, hygiene
make hooks      # install the commit hooks: rustfmt, the ratchet, the fast hygiene gates
make test       # cargo test --workspace
make metadata   # regenerate and verify generated artifacts
make wasm       # build every release crate for wasm32-unknown-unknown
make bench      # benchmarks (report-only)
make build-profile-hygiene  # prove the gate is compiled the way it claims
```

`make check` stays a pure-Rust gate. `scale-corpus`, `lubm` and `watdiv` are
report-only comparison lanes ([`docs/BENCHMARKS.md`](./docs/BENCHMARKS.md)).

**Verification is focused.** During development select the affected behavior,
test, crate or real consumer, with the profile, target and warnings the gate
uses. Whole-workspace runs (`make check`, `make test`) are for a mandatory gate
or hook, integration work that needs them, or an explicit request: never a
ritual after each change or before each report. Commit hooks run fully and are
never bypassed; do not run by hand what the hook is about to run. `make check`,
conformance, rustdoc, the WASM build and host execution establish different
facts, and one does not stand in for another.

**Toolchain.** `rust-toolchain.toml` names a floating nightly for development
and ordinary CI, for its sharper lints. The source stays free of nightly
features: no `#![feature(...)]`. The MSRV is `rust-version` in `Cargo.toml`, a
lower floor on stable, checked by the `msrv` job; never align it with the
development channel. Release lanes build on stable. Only `RUSTUP_TOOLCHAIN`
outranks the toolchain file, which is why those jobs set it
(`scripts/check-toolchain-pin.py`). Do not change compiler selection to hide a
diagnostic.

## 4. Performance discipline

PurRDF is a hot-path backbone over an immutable, value-interned dataset.

* **Measure first.** A claimed improvement is shown against the same behavioral
  contract, on the shared harness (`purrdf_testkit::bench`,
  `purrdf_alloc_probe`), by work counters, allocation counts, requested and
  retained bytes, with host and build conditions stated. Add or extend a bench
  when you claim a win. A single run must not get dearer for work that only
  pays back on a later one.
* Avoid a `String` per token or term; move values out of buffers instead of
  cloning; size collections before parse loops. Hot maps use the fixed-key
  hasher.
* **The gate is compiled like production.** `[profile.dev]` builds everything
  at `opt-level = 3` with debug assertions and overflow checks on; those two
  are not negotiable. `[profile.test]` sets only `debug = 0`. Neither profile
  sets `lto` or `codegen-units = 1`. `make build-profile-hygiene` checks the
  effective profile of every unit Cargo resolves, not the manifest.

## 5. Brand & naming

The project name is written **PurRDF** in prose (never PurrDF/PURRDF); all
package/crate/binary identifiers are lowercase `purrdf`. See
[`docs/BRAND.md`](./docs/BRAND.md).

## 6. Releases

Tag-driven trusted publishing: `rust-v*` → crates.io (40 crates, ordered),
`py-v*` → PyPI (`purrdf`). See [`docs/RELEASE.md`](./docs/RELEASE.md). Version
is single-sourced in `[workspace.package]`. Ten members never reach
crates.io: `purrdf-capi`, `purrdf-sparql-conformance`,
`purrdf-shacl-conformance`, `purrdf-hash-conformance`, `purrdf-cli`,
`purrdf-envelope-probe`, `purrdf-bench`, `wasm-link`, `helper-census`, and
`purrdf-python` (PyPI via maturin instead). The published development libraries
`purrdf-alloc-probe`, `purrdf-testkit` and `purrdf-conformance-kit` keep explicit
versions and precede their consumers in publication order. Tagging and
publishing are the owner's decision; an agent prepares, and does not tag.

## 7. Provenance

This repository was extracted from `gmeow-ontology` and `gmeow-gts`; see
[`PROVENANCE.md`](./PROVENANCE.md). The downstream `gmeow-ontology` cutover is
not complete: its legacy models are migration evidence only and are deleted as
each PurRDF replacement is integrated.

---

## Rust engineering

The detailed guide is
[docs/agents/RUST_GUIDELINES.md](./docs/agents/RUST_GUIDELINES.md); read it
before writing or reviewing Rust. Its core, as it applies here:

* **Make illegal states unrepresentable.** Parse into a type at the boundary
  and pass that type, not a raw string or integer with domain meaning. Use
  `let ... else` for early returns. Accept `&str`, slices and borrowed views;
  own only when the value must outlive its producer.
* **Visibility is a commitment.** Default to private, then `pub(crate)`. Every
  `pub` item is a consumer contract under the compatibility baseline.
* **Thin entry points, thick libraries.** Bindings and commands adapt inputs,
  ownership and output transport to the one engine.
* **Shared structures and constructors.** Reach for the workspace's own before
  writing one: `purrdf_lex::walk` (`WorkList`, `Nested`, `write_debug`) for
  whole-tree walks without machine-stack recursion, `purrdf_lex::assoc` for
  ordered association lists, `purrdf_lex::json_record!` for record codecs, and
  `purrdf_core::hash::table` for fixed-key tables. A type whose empty value is
  the derived default derives `Default` and has no trivial `new()`; keep
  `new()` with `purrdf_hash::default_from_new!` only when it must be `const`
  or differs. A constructor that is one conversion uses
  `purrdf_lex::constructors!`, a `From` that wraps a source into one variant
  uses `purrdf_lex::variant_from!`, and an error that is one message uses
  `purrdf_lex::message_error!`.
* **Errors are typed.** Keep semantic failure data in the error and keep the
  native cause (`source`). Inspect through `purrdf_hash::error`
  (`StructuredError`, `error_visit!`); render with `Display` only at the
  presentation boundary. Propagate with `?`. No `unwrap`, `expect` or panic for
  invalid input or resource exhaustion in production paths; an unavoidable
  invariant check carries a local proof. No typed failure is replaced by an
  infallible allocation: admit storage through the allocation interfaces and
  keep the refusal.
* **No incidental output.** Libraries return values, bytes, diagnostics and
  evidence. No `println!`, `eprintln!` or `dbg!` in an engine path. The command
  tools write their results by design; keep their protocol.
* **Unsafe is narrow.** Ordinary engine code is safe. FFI, SIMD and host
  imports carry a local safety argument for each operation, and dispatch
  establishes the instruction set before a target-specific kernel runs.
* **Exact arithmetic stays exact.** Integer, decimal, rational and temporal
  operations use their shared homes; a float never silently replaces an exact
  result.
* **Tests earn their keep.** A permanent test runs on every change forever, so
  it protects a named production behavior, regression, boundary or invariant,
  and says which. No tests of getters, compiler guarantees, mechanical
  forwarding or an implementation's shape; no mocks that assert a call
  sequence; no mutation testing. Prefer one property (`prop_test!`) to many
  copied cases, and a byte-exact golden (`assert_golden!`) to a hand-written
  comparison. Remove tests that are outdated or overlap.
* **WASM tests are for WASM.** General semantics, grammar, arithmetic, refusal,
  ownership and conformance run natively. WASM execution is limited to real
  dispatch or SIMD kernels, shadow-stack floors, host interfaces and named
  frozen byte or digest identities, each named in
  [docs/WASM_TESTING.md](./docs/WASM_TESTING.md), running only its exact cases.
  A portable build, a 32-bit integer or an existing selection is not a reason.
