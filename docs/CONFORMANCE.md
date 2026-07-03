<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Conformance

Every PurRDF engine is gated by its official test suite. The suites are
vendored and **frozen** in-repo (never hand-edited), the harnesses assert
**exact counts** in both directions (a new failure breaks the build, and so
does a silently-fixed expected-failure — XPASS discipline), and every
expected-failure ledger entry carries a precise reason, so the ledgers double
as roadmaps.

## Single conformance matrix

The native Rust W3C harnesses **and** the Python rdflib drop-in gate are
reported together as one scoreboard: all suites green and reported in CI as a
single conformance matrix, with every gap skip-listed with a reason and never
silently passed. Run it locally:

```sh
make conformance            # aggregates every suite below into one table
```

The aggregator (`scripts/conformance-matrix.py`) runs each suite in a fixed
order, scrapes its own scoreboard line, and prints per-suite
**pass / xfail-or-skip / fail** counts with an overall **RED/GREEN** verdict. It
exits non-zero on any unexpected failure — a red run, an XPASS, or a stale
ledger key — and, under CI, appends the matrix to the job summary via
`$GITHUB_STEP_SUMMARY` (see the `conformance` job in
[`.github/workflows/ci.yaml`](../.github/workflows/ci.yaml)). It never mutates a
frozen vector or weakens a gate; it is a pure reporter over the existing
harnesses.

Latest measured matrix (`make conformance`, all GREEN, exit 0):

| Suite | Source | Pass | XFail/Skip | Fail |
| --- | --- | ---: | ---: | ---: |
| IRI (RFC 3987 / RFC 3986 resolution) | W3C IRI + RFC vectors | 19 | 0 | 0 |
| RDFC-1.0 canonicalization | W3C rdf-canon | 6 shards | 0 | 0 |
| Syntax codecs (Turtle/TriG/NT/NQ/RDF-XML) | W3C rdf-tests | 250 | 0 | 0 |
| SPARQL 1.1/1.2 evaluation (full corpus) | W3C sparql11 + sparql12 + first-party | 437 | 213 | 0 |
| SHACL Core + SHACL-SPARQL | W3C data-shapes | 114 | 6 | 0 |
| SHACL (first-party corpus) | first-party frozen reports | 48 | 0 | 0 |
| ShEx 2.1 validation | shexTest v2.1.0 | 1,051 | 54 | 0 |
| ShEx syntax + ShExC/ShExJ round-trip | shexTest v2.1.0 | 9 groups | 0 | 0 |
| rdflib LSP drop-in gate | rdflib 7.6 own tests | 63 | 23 | 0 |
| purrdf.compat parity | first-party (differential vs rdflib) | 325 | 7 | 0 |

The RDFC, SHACL-corpus, and ShEx-syntax rows count harness test functions
(each fans out over its fixtures internally); the fixture-level totals are in
the per-engine scoreboard below. The XFail/Skip column is always a **ledgered**
number, never a silent skip (see [Ledger discipline](#ledger-discipline) and
[Known gaps](#known-gaps)).

## Scoreboard (per engine)

| Engine | Suite | Result |
| --- | --- | --- |
| IRI (RFC 3987) | W3C IRI + RFC 3986 §5.4 resolution vectors | parse/validate/normalize/resolve, green |
| ShEx 2.1 validation | shexTest v2.1.0, `validation/` | **1,051 / 1,051** attempted · 0 xfail · 54 trait-skips (Import 32, SemanticAction 22) |
| ShEx schemas (ShExC ∥ ShExJ) | shexTest v2.1.0, `schemas/` | **425/425** ShExC parse · **420/420** ShExJ round-trip · 419/420 ShExC≡ShExJ AST (1 upstream corpus bug, documented) |
| ShEx negative syntax | shexTest v2.1.0, `negativeSyntax/` | **99 / 99** rejected |
| ShEx negative structure | shexTest v2.1.0, `negativeStructure/` | **14 / 14** rejected |
| SHACL | W3C data-shapes, `core/` + `sparql/` | **114 / 120** · 6 ledgered |
| SHACL (first-party corpus) | `crates/shapes/corpus/` | **48 / 48** frozen expected reports |
| Syntax codecs | W3C rdf-tests `crates/rdf/tests/corpus/w3c/` | **250 / 250** round-trip (nquads 27, ntriples 29, rdfxml 31, trig 60, turtle 103) · 0 gaps |
| SPARQL 1.1/1.2 | full W3C sparql11 (query+update) + sparql12 + entailment, via `purrdf-sparql-conformance` | **437** pass · 213 typed xfail · 0 fail (all W3C `service` federation cases green; SPARQL 1.1 query+update fully vendored; SPARQL 1.2 vendored as a first-class spec, residual RDF-star features tracked) |
| Entailment (RDFS / OWL-RL) | native `purrdf-entail` forward-materialization reasoner | RDFS + OWL-RL closure; **39/70** W3C entailment cases (OWL-Direct/DL/RIF/D ledgered) |
| RDFC-1.0 canonicalization | W3C fixtures, `crates/rdf/tests/fixtures/rdfc/` | **65** vectors (64 eval + 1 negative), green |
| rdflib drop-in (LSP) gate | rdflib 7.6 own vendored tests | **63** pass · 23 strict-xfail (ledgered) |
| purrdf.compat parity | first-party differential vs rdflib 7.6 | **325** pass · 7 strict-xfail (ledgered) |
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
- `crates/rdf/tests/corpus/w3c/` — the W3C rdf-tests syntax corpus (Turtle,
  TriG, N-Triples, N-Quads, RDF/XML) driving the native-codec round-trip gate.
- `crates/rdf/tests/fixtures/rdfc/` — the W3C rdf-canon (RDFC-1.0) vectors.
- `crates/iri/tests/` — the IRI/URI validity vectors and RFC 3986 §5.4
  resolution examples.
- `bindings/python/tests/rdflib_suite/vendor/` — rdflib 7.6's own tests,
  vendored verbatim and run against the purrdf drop-in.
- `vectors/*.gts` + subdirs — the GTS conformance corpus shared byte-exact
  with the other GTS engines (governed in
  [`gmeow-gts`](https://github.com/Blackcat-Informatics/gmeow-gts)).

## Running them

```sh
make conformance                                      # the single matrix (all of the below + the rdflib gate)

cargo test -p purrdf-iri                               # IRI + RFC 3986 resolution
cargo test -p purrdf-shex                              # all four ShEx suites
cargo test -p purrdf-shapes --test w3c_conformance -- --nocapture   # W3C SHACL scoreboard
cargo test -p purrdf-shapes --test conformance         # the 48-case frozen corpus
cargo test -p purrdf-sparql-conformance                # W3C SPARQL
cargo test -p purrdf-rdf                               # RDFC-1.0 + codec goldens
cargo test -p purrdf-gts                               # GTS vectors
cd bindings/python && uv run pytest tests/test_rdflib_suite.py -q   # rdflib drop-in gate
```

`make check` runs the Rust suites as part of the workspace gate; `make pytest`
runs the Python gate; `make conformance` runs the conformance slices of both as
one reported matrix.

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

The ledgers themselves (each entry = one node id + a concrete, self-describing
reason):

- `bindings/python/tests/xfail_ledger.toml` — the first-party
  `purrdf.compat` parity ledger (7 strict xfails).
- `bindings/python/tests/rdflib_suite/xfail_ledger.toml` — the rdflib
  drop-in (LSP) gate ledger governing rdflib's own vendored tests (23 strict
  xfails). Both are applied as **strict** xfails, so an XPASS or a stale key
  fails the run.
- The Rust harnesses embed their ledgers in-code (e.g. the SHACL `w3c_conformance`
  xfail table, the SPARQL `purrdf-sparql-conformance` `xfail` module, the codec
  allowlist) and assert each entry still fails.

## Known gaps

These are **tracked, never silent** — each is a ledgered xfail/skip or an open
issue, so the matrix stays honest:

- **SPARQL 1.1 eval** — the full W3C SPARQL 1.1 **query + update** evaluation
  suites are vendored verbatim (`crates/sparql-conformance/suite/w3c-sparql11/`,
  pinned commit `426c7df`) and every non-pass is a typed strict-xfail in
  `crates/sparql-conformance/src/xfail.rs`. The residuals: `value-mismatch`
  (XSD cast + numeric/string-function lexical/datatype form), `property-path`
  (inverse-in-NPS, `*`/`?` over property sets), `update-semantics` (COPY/ADD
  edge cases, cross-op blank-node scoping), `unsupported-construct`
  (`CONSTRUCT WHERE`, grouping validation, EXISTS-over-GRAPH-var),
  `non-deterministic` (`BNODE()` labels), and `result-format` (Turtle
  `rs:ResultSet`). **All 7 W3C `service` federation cases pass** (via the lateral
  SERVICE seam + trailing-`VALUES` parser fix).
- **SPARQL 1.2 / RDF-1.2** — a complete, first-class spec here (RDF-star: triple
  terms, reifiers, base direction), vendored under `suite/w3c-sparql12/`
  (SHA-pinned for reproducibility/errata, the same as every other suite). The
  surface the engine already satisfies passes; the residual triple-term/reifier
  grammar and evaluation are `parse-unsupported`/`unsupported-construct` xfails —
  **genuine unimplemented features tracked until they land**, not
  provisional-spec placeholders.
- **Entailment** — the native `purrdf-entail` reasoner materializes RDFS +
  OWL-RL closure; RDF/RDFS/OWL-RL cases pass, and OWL-Direct(DL)/RIF/D-entailment
  cases are `entailment` xfails (spec-inherent boundaries of a
  forward-materialization reasoner).
- **SHACL** — 6 W3C `sparql/` cases are ledgered.
- **ShEx Import / SemanticAction** — 54 shexTest cases are trait-skipped.
- **rdflib drop-in residuals** — 23 rdflib-suite + 7 compat-parity strict
  xfails cover Graph-subclass identity through set operators, rdf:List /
  Collection mutation, `Result.bindings` / `SELECT *` subselect projection,
  graph-prefix forwarding, aggregate/nested-FILTER evaluation, and
  ConjunctiveGraph legacy semantics; see the two ledgers above for the per-test
  reasons.

## Comparison caveats

- The SHACL harness compares the result **multiset** on
  `(focusNode, resultPath, value, sourceConstraintComponent, severity)`;
  `sh:resultMessage` text and nested `sh:detail` are not compared.
- ShEx logic-conformance (pass/fail parity) is the reported level, per suite
  convention; result-structure conformance is upstream-experimental.
- One shexTest schemas entry (`start2RefS2`) has a frozen ShExJ that
  contradicts its own ShExC source; the harness pins our (correct) reading
  and documents the divergence.
