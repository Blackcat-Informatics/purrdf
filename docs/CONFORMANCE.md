<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Conformance

Every PurRDF engine is gated by its official test suite. The suites are
vendored and **frozen** in-repo (never hand-edited), the harnesses assert
**exact counts** in both directions (a new failure breaks the build, and so
does a silently-fixed expected-failure — XPASS discipline), and every
expected-failure ledger entry carries a precise reason, so the ledgers double
as roadmaps.

## Scoreboard

| Engine | Suite | Result |
| --- | --- | --- |
| ShEx 2.1 validation | shexTest v2.1.0, `validation/` | **1,051 / 1,051** attempted · 0 xfail · 54 trait-skips (Import 32, SemanticAction 22 — [#14](https://github.com/Blackcat-Informatics/purrdf/issues/14)) |
| ShEx schemas (ShExC ∥ ShExJ) | shexTest v2.1.0, `schemas/` | **425/425** ShExC parse · **420/420** ShExJ round-trip · 419/420 ShExC≡ShExJ AST (1 upstream corpus bug, documented) |
| ShEx negative syntax | shexTest v2.1.0, `negativeSyntax/` | **99 / 99** rejected |
| ShEx negative structure | shexTest v2.1.0, `negativeStructure/` | **14 / 14** rejected |
| SHACL | W3C data-shapes, `core/` + `sparql/` | **114 / 120** · 6 ledgered ([#12](https://github.com/Blackcat-Informatics/purrdf/issues/12), [#13](https://github.com/Blackcat-Informatics/purrdf/issues/13)) |
| SHACL (first-party corpus) | `crates/shapes/corpus/` | **48 / 48** frozen expected reports |
| SPARQL 1.1 | W3C suite via `purrdf-sparql-conformance` | green, xfail-ledgered |
| RDFC-1.0 canonicalization | W3C fixtures, `crates/rdf/tests/fixtures/rdfc/` | green |
| GTS transport | frozen cross-language vectors, `vectors/` | byte-exact |

## Where the suites live

- `vectors/shexTest/` — the official ShEx suite pinned at tag `v2.1.0`
  (upstream `main` has drifted to 2.2-alpha `EXTENDS` tests, out of scope for
  ShEx 2.1). See its README for provenance.
- `vectors/shacl/` — the W3C SHACL test suite (`data-shapes-test-suite`),
  `core/` and `sparql/` manifests. See its README for provenance.
- `crates/shapes/corpus/` — PurRDF's own frozen SHACL corpus: 48 cases with
  byte-frozen expected reports, covering purrdf-specific behavior (reifier
  shapes, path forms, property pairs, qualified shapes).
- `crates/sparql-conformance/` — the W3C SPARQL 1.1 harness plus first-party
  extension-function and standpoint suites.
- `vectors/*.gts` + subdirs — the GTS conformance corpus shared byte-exact
  with the other GTS engines (governed in
  [`gmeow-gts`](https://github.com/Blackcat-Informatics/gmeow-gts)).

## Running them

```sh
cargo test -p purrdf-shex                              # all four ShEx suites
cargo test -p purrdf-shapes --test w3c_conformance -- --nocapture   # W3C SHACL scoreboard
cargo test -p purrdf-shapes --test conformance         # the 48-case frozen corpus
cargo test -p purrdf-sparql-conformance                # W3C SPARQL
cargo test -p purrdf-rdf                               # RDFC-1.0 + codec goldens
cargo test -p purrdf-gts                               # GTS vectors
```

`make check` runs all of the above as part of the workspace gate.

## Ledger discipline

A harness never skips silently. The three mechanisms:

1. **Exact totals** — each harness asserts the number of discovered tests, so
   corpus drift (a deleted or unreachable manifest entry) fails loudly.
2. **XFAIL ledgers** — known gaps are listed with a reason string; the
   harness asserts each still fails. Fixing one without removing its entry
   breaks the build ("XPASS"), which keeps the ledger honest.
3. **Trait skips (ShEx only)** — whole spec features staged for later
   (imports, semantic actions) are skipped by their manifest trait tags and
   counted exactly; nothing else may be skipped.

## Comparison caveats

- The SHACL harness compares the result **multiset** on
  `(focusNode, resultPath, value, sourceConstraintComponent, severity)`;
  `sh:resultMessage` text and nested `sh:detail` are not compared.
- ShEx logic-conformance (pass/fail parity) is the reported level, per suite
  convention; result-structure conformance is upstream-experimental.
- One shexTest schemas entry (`start2RefS2`) has a frozen ShExJ that
  contradicts its own ShExC source; the harness pins our (correct) reading
  and documents the divergence.
