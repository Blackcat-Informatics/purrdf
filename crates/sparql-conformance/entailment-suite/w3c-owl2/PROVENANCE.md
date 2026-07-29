<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Vendored W3C OWL 2 conformance corpus

This tree vendors the **W3C OWL 2 test suite**, pre-flattened to one directory
per test case. It is consumed by the native OWL 2 grader
(`crates/sparql-conformance/src/owl2.rs`, driven by
`crates/sparql-conformance/tests/owl2_conformance.rs`), which decides each case's
consistency through PurRDF's `OWL-Direct` ALCOIQ tableau and grades the answer
against the verdict the W3C published for it.

## What this corpus does and does NOT validate

**It validates the DL / tableau lane's verdicts, and nothing else.**

Every one of the 261 vendored cases is a `otest:ConsistencyTest` or an
`otest:InconsistencyTest`: the published ground truth is a *satisfiability*
verdict over an ontology, decided here by `purrdf_entail::materialize_dl`. There
is not one `otest:PositiveEntailmentTest` or `otest:NegativeEntailmentTest` in
the tree (see *What was left behind* below for why).

It therefore does **not** validate the OWL 2 RL rule table. PurRDF's OWL 2 RL
lane is a forward-materialization chase over a declared rule program, exercised
by authored per-rule fixtures in `crates/entail`; nothing in this corpus touches
it. A reader looking at the `Entailment` row of the conformance matrix is looking
at open-world DL consistency, not at rule coverage.

## Source

- Upstream: the **W3C OWL 2 test suite**, exported as a single RDF/XML manifest
  at <https://www.w3.org/2009/11/owl-test/all.rdf>.
- The suite is a static W3C archive published under `/2009/11/owl-test/` and has
  no upstream VCS revision to pin; its archive revision is recorded upstream as
  **`w3c-2009-11-archive`**, and that is the pin used here.
- The cases were **not** re-derived from `all.rdf` in this repository. They were
  taken from the already-flattened, already-audited copy in the sibling
  `Blackcat-Informatics/gmeow-ontology` repository, at
  `conformance/logic/cases/external/w3c-owl2-full/`, pinned at commit
  **`8906e41b15d5adaeccede35dab7e36c7eab86147`** (that path last changed on
  2026-07-08).
- Fetched into this repository on **2026-07-29**.

## Fidelity — file by file

| File | Fidelity |
|------|----------|
| `cases/<case>/source/premise.rdf` | **verbatim**, byte-for-byte, as published by the W3C. The flattening extracted each test's `otest:rdfXmlPremiseOntology` literal and wrote it out unmodified: no SPDX header was injected, no namespace was rewritten, no whitespace was normalized. |
| `cases/<case>/profile.json` | **derived**, mechanically, by the flattening. `w3c_published_verdict` is the suite's own classification of the case: `consistent` for an `otest:ConsistencyTest`, `inconsistent` for an `otest:InconsistencyTest`. The remaining keys (`mode`, `native_verdict`, `verdict_mode`) are the flattening tool's own bookkeeping and are **ignored** by this repository's grader — in particular `native_verdict` is a *different* reasoner's answer and is never treated as ground truth here. |

Every file is a byte-exact copy of its counterpart in the pinned source above.
Nothing in this tree has been hand-edited, and the byte-freeze gate
(`scripts/check-corpus-frozen.py`, manifest
`scripts/conformance-frozen/sparql-conformance-w3c-owl2.sha256`) makes that
claim enforceable rather than aspirational.

### Why the upstream `manifest.ttl` is not vendored

The flattened source also carries a per-case `source/manifest.ttl` naming the
`otest:` test type. It is **not** vendored here, for two reasons:

1. It is not verbatim W3C material — the flattening synthesized it, injecting an
   SPDX header and rewriting the test IRIs into a namespace belonging to the
   sibling project.
2. That injected header declares the bare SPDX id `W3C`, which denotes the W3C
   *Software* Notice and License — a different document from the W3C Test Suite
   License these files are actually published under. Vendoring 261 files that
   each declare the wrong identifier would satisfy the license-hygiene gate while
   making the tree's licensing statement false.

The one fact the manifest carries that the grader needs — the published verdict —
is preserved in `profile.json`'s `w3c_published_verdict`, and the two agree on
all 261 cases (226 `ConsistencyTest` → `consistent`, 35 `InconsistencyTest` →
`inconsistent`).

## Base IRI

Each `premise.rdf` is parsed by PurRDF's first-party RDF/XML codec with a
synthetic, deterministic base IRI (`http://example.org/w3c-owl2/<case>` — an
`example.org` fixture IRI, per the repository's no-fabricated-vocabulary rule).
That base is never actually consulted: **0 of the 261 vendored premises contain a
relative RDF/XML reference (`rdf:about` / `rdf:resource` / `rdf:ID` /
`rdf:datatype`) without also declaring their own `xml:base`**, so every IRI in
every case resolves from the document itself and no verdict depends on the
harness's choice of base.

## What was vendored, and what was left behind

The flattened source holds five W3C OWL 2 buckets. Exactly one is vendored here:

| Upstream bucket | Cases | Vendored? | Why |
|-----------------|------:|-----------|-----|
| `w3c-owl2-full` | 261 | **yes** | The mainline bucket of `all.rdf`, taken whole. 226 consistency + 35 inconsistency cases, 202 KB, graded end-to-end in ~180 ms in a debug build. |
| `w3c-owl2-el` | 19 | no | All 19 case names are, name-for-name, a subset of `w3c-owl2-full`. Vendoring them would duplicate payload for zero additional coverage. |
| `w3c-owl2-full-decided` | 32 | no | Contains premises on which PurRDF's ALCOIQ tableau does not terminate inside any budget a required gate can carry: `webont-i5-8-001` was still running after eight minutes of a debug-build grade before the probe was stopped. A conformance row that cannot finish is not a conformance row. |
| `w3c-owl2-full-divergence` | 122 | no | 3.9 MB — roughly twenty times the vendored payload — dominated by `webont-description-logic-*` premises of 100–220 KB each. Left out under the size discipline that governs this tree. |
| `w3c-owl2-el-divergence` | 2 | no | Two cases from the same triage bucket as `w3c-owl2-full-divergence`, excluded with it. |

### Why there are no entailment tests here

The `Entailment` conformance row is backed entirely by consistency cases. That is
not a choice of slice — it is what the W3C material in the flattened source
contains. Across all five buckets above (436 cases) the `otest:` test type is
`ConsistencyTest` or `InconsistencyTest` and never `PositiveEntailmentTest` or
`NegativeEntailmentTest`.

The flattened source *does* carry six entailment-shaped cases (two positive, two
negative, plus two divergence cases), but they are **self-authored fixtures of
the sibling project**, published under CC-BY-4.0 with an upstream of
`gmeow-self-authored` — not W3C material. Vendoring them under this tree's
`LicenseRef-W3C-Test-Suite` declaration would misstate their provenance and their
license, so they are out of scope for a W3C OWL 2 corpus.

## License

The W3C OWL 2 test files are published under the **W3C Test Suite License** /
**W3C Software and Document License** — see
<https://www.w3.org/Consortium/Legal/2015/copyright-software-and-document>. They
are vendored verbatim and are **not** relicensed. Rather than 522 per-file
`.license` sidecars, the tree declares them with two `REUSE.toml` glob
annotations (`REUSE.toml`), both naming `LicenseRef-W3C-Test-Suite`; the license
text is in `LICENSES/LicenseRef-W3C-Test-Suite.txt`. This document and the
grader that reads the tree are PurRDF-authored (MIT OR Apache-2.0).

## Re-vendoring

1. Refresh the flattened source in the sibling repository from upstream:

   ```sh
   curl -sSL https://www.w3.org/2009/11/owl-test/all.rdf -o .tmp/w3c-owl2/all.rdf
   ```

   then re-run that repository's own `--vendor-full` flattening over it.
2. Copy `<case>/profile.json` and `<case>/source/premise.rdf` for every case of
   the `w3c-owl2-full` bucket into `cases/` here, preserving bytes.
3. Update the *Source* section above with the new pin and fetch date.
4. Regenerate the byte-freeze manifest and re-measure the ledger:

   ```sh
   python3 scripts/check-corpus-frozen.py --update
   cargo test -p purrdf-sparql-conformance --locked --test owl2_conformance -- \
       --ignored --nocapture regenerate_ledger
   ```

   The second command prints a paste-ready replacement for the `LEDGER` table in
   `crates/sparql-conformance/src/owl2.rs`. Every entry it emits must be given a
   typed reason by hand — an unexplained divergence is not a ledger entry.
