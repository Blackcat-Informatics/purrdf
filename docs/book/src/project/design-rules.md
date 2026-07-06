<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Design Rules & Invariants

PurRDF must stay fast, deterministic, and boring: **one engine, one behavior,
carried verbatim into Rust, Python, WebAssembly, and C**. That promise is kept
by a small set of hard invariants, each enforced by CI rather than by
convention. The canonical statement is
[AGENTS.md](https://github.com/Blackcat-Informatics/purrdf/blob/main/AGENTS.md)
in the repository; this chapter explains the *why*.

## No Cargo features, ever

The workspace has **zero feature flags**, and a CI script gates it. PurRDF is
a data carrier, and optionality changes semantics per consumer — two builds of
"the same version" that parse or serialize differently would defeat the whole
point. No `[features]`, no optional dependencies, no `cfg`-gated behavior
differences. Every consumer gets the same byte-identical semantics.

## PurRDF is NOT an ontology — it mints no vocabulary IRIs

Every vocabulary the library reads or writes — slice manifests,
statement-metadata downcast, box roles, SPARQL extension-function namespaces,
JSON-Schema namespaces — is **caller-supplied configuration with no
fabricated default**. A feature exercised without its vocabulary hard-errors
or stays inactive. The library never hardcodes a vendor namespace (the GMEOW
ontology is a *consumer*; the dependency arrow never points from purrdf to
it), and test fixtures use `example.org`. Consumer-config types (`SliceVocab`,
`Namespaces`, `StatementMetadataVocab`) are unified behind an
`OntologyProfile` a downstream builds once.

## Byte determinism

Serializers and the GTS writer are byte-deterministic. No iteration-order,
time, or RNG dependence is permitted in any output path; hot maps use
fixed-key `ahash` for this reason. Changes that alter emitted bytes must
update the affected golden files, visibly.

## The kernel ring-fence

`purrdf-core` must never depend on oxigraph or PyO3 — the whole workspace is
oxigraph-free, and a hygiene gate asserts the dependency tree. The three
foundation leaves (`purrdf-iri`, `purrdf-xsd`, `purrdf-events`) keep **zero
runtime dependencies**. Diagnostics stay structured and SARIF-free in the
kernel; the SARIF boundary is the `purrdf-validate` leaf.

## Everything is wasm-able

Every release crate must build for `wasm32-unknown-unknown`, and CI
hard-fails otherwise. No dependency may drag in threads, the filesystem, C
toolchains, or wall-clock/RNG syscalls on the wasm path — cryptography stays
pure Rust for exactly this reason. This is what makes the JavaScript package
the *same engine* rather than a port.

## Hard-fail, never wrong

Across the toolkit, out-of-scope input is a **typed error**, never a partial
answer: malformed RDF is an `RdfDiagnostic`, an unsupported SPARQL builtin is
`EvalError::Unsupported`, a malformed ShEx schema is a `ShexError`,
D-entailment is `EntailError::Unsupported`, and an unsupported results
projection is a typed format error. Lossy-by-design projections are permitted
but *loud*, via the [loss ledger](../slices.md#the-loss-ledger).

## Conformance corpora are the contract

The W3C and community test suites are vendored, **byte-frozen**, and
SHA-256-verified; harnesses assert exact counts and enforce XPASS discipline
on their expected-failure ledgers. See
[Conformance & Testing](conformance.md).

## Supporting rules

- **Measured performance** — perf claims require a criterion bench, not an
  adjective ([Performance](performance.md)).
- **One version, lockstep releases** — crates.io, PyPI, and npm ship one
  workspace version ([Versioning & Releases](releases.md)).
- **Stable toolchain only** — the workspace is nightly-free; the MSRV floor
  (currently 1.96) is enforced by a dedicated CI job.
- **Brand** — the project is **PurRDF** in prose and `purrdf` in identifiers
  ([`docs/BRAND.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/BRAND.md)).
