<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Vendored W3C OWL 2 conformance corpus

This tree vendors the **W3C OWL 2 test suite**, pre-flattened to one directory
per test case. It is consumed by the native OWL 2 grader
(`crates/sparql-conformance/src/owl2.rs`, driven by
`crates/sparql-conformance/tests/owl2_conformance.rs`), which decides each case's
consistency through PurRDF's `OWL-Direct` SHOIQ(D) tableau and grades the answer
against the verdict the W3C published for it.

## What this corpus does and does NOT validate

**It validates the DL / tableau lane's verdicts, and nothing else.**

Every one of the 261 vendored cases is a `otest:ConsistencyTest` or an
`otest:InconsistencyTest`: the published ground truth is a *satisfiability*
verdict over an ontology, decided here by `purrdf_entail::materialize_dl_reported`. There
is not one `otest:PositiveEntailmentTest` or `otest:NegativeEntailmentTest` in
**this tree** — because none was vendored into it, *not* because the W3C material
lacks them (see *Correction* below).

It therefore does **not** validate the OWL 2 RL rule table. PurRDF's OWL 2 RL
lane is a forward-materialization chase over a declared rule program; nothing in
this corpus touches it. That lane is graded against W3C's own entailment tests in
the sibling tree `../w3c-owl2-rl/`. A reader looking at the `Entailment` row of
the conformance matrix is looking at open-world DL consistency, not at rule
coverage.

It is also a **subset**: 261 of the 482 consistency-shaped cases the upstream
manifest publishes. See *What was left behind*.

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
| `cases/<case>/source/premise.rdf` | **near-verbatim**: each test's `otest:rdfXmlPremiseOntology` literal, with **trailing whitespace stripped from every line and a final newline appended** by the upstream flattening. No SPDX header was injected and no namespace was rewritten, but the bytes are not the W3C's bytes. Re-extracting all 261 premises from `all.rdf` reproduces this tree byte-for-byte only under that normalization: **0 of 261 match raw, 261 of 261 match normalized**. (An earlier revision of this document claimed "byte-for-byte … no whitespace was normalized". That was wrong, and it is corrected here. The sibling tree `../w3c-owl2-rl/` vendors the literal values with no normalization at all.) |
| `cases/<case>/profile.json` | **derived**, mechanically, by the flattening. `w3c_published_verdict` is the suite's own classification of the case: `consistent` for an `otest:ConsistencyTest`, `inconsistent` for an `otest:InconsistencyTest`. The remaining keys (`mode`, `native_verdict`, `verdict_mode`) are the flattening tool's own bookkeeping and are **ignored** by this repository's grader — in particular `native_verdict` is a *different* reasoner's answer and is never treated as ground truth here. |

Every file is a byte-exact copy of its counterpart in the *pinned sibling source*
above — which is not the same thing as a byte-exact copy of what W3C published,
as the row above records. Nothing in this tree has been hand-edited, and the
byte-freeze gate
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
| `w3c-owl2-full-decided` | 32 | no | Named for premises on which PurRDF's SHOIQ(D) tableau once did not terminate inside any budget a required gate could carry. That is **measured** rather than inherited — see *The exclusions, measured* below — and the current measurement finds **0** non-terminating cases at a 40 s ceiling: `webont-i5-8-001`, the case an earlier revision of this bucket was named for, now decides. The bucket is still excluded on size grounds, not on the termination grounds that motivated it originally. |
| `w3c-owl2-full-divergence` | 122 | no | 3.9 MB — roughly twenty times the vendored payload — dominated by `webont-description-logic-*` premises of 100–220 KB each. Left out under the size discipline that governs this tree. |
| `w3c-owl2-el-divergence` | 2 | no | Two cases from the same triage bucket as `w3c-owl2-full-divergence`, excluded with it. |

### The exclusions, measured

The bucket table above describes the *flattened source*. Measured against the
**upstream manifest**, this tree vendors 261 of the 482 consistency-shaped cases
W3C publishes. The other **221** were, until now, invisible: the harness reported
`agreed 256 / total 261` over a set the hard cases had been removed from.

They are invisible no longer. Every one of them is named, with its measured
disposition, in `../w3c-owl2-rl/census.tsv` (`dl_corpus` / `dl_probe` columns),
the `owl2_conformance` harness prints the tally and the non-terminating names on
every run as `OWL2-DL-EXCLUDED`, and both totals are pinned as constants so a case
cannot enter or leave the exclusion set unnoticed:

| Disposition | Cases |
|-------------|------:|
| the tableau **cannot decide** it (no answer within a 40 s per-case ceiling) | **0** |
| the tableau decides it today (would grade if vendored) | 173 |
| the run withholds with an honest error (step cap, unread construct, codec refusal) | 25 |
| no `otest:rdfXmlPremiseOntology` at all (functional syntax only) | 23 |
| **total excluded** | **221** |

Measured 2026-08-10, release build, one process per case, four concurrent, 40 s
wall-clock ceiling each — re-measured from the 2026-07-29 debug-build figures
(30 / 156 / 12 / 23) after clausification and search-refinement work changed
what the tableau does with every one of these premises. The headline is the
second row: **173 of the 221 exclusions are cases the reasoner decides** (up
from 156), and the non-terminating row is now empty — every case that used to
exhaust its wall-clock budget resolves one way or the other today. The
exclusion remains a payload-size and triage decision and not a capability
limit, and the 261 denominator understates neither the reasoner nor overstates
it by accident — it simply is not the W3C denominator, and the harness now
says so out loud. See `../w3c-owl2-rl/PROVENANCE.md`'s census section for the
two decided cases whose verdict direction moved between the two
measurements.

### Correction: there ARE upstream entailment tests

An earlier revision of this document said, under the heading *"Why there are no
entailment tests here"*, that the absence of entailment cases "is what the W3C
material in the flattened source contains", and used that to justify grading the
OWL 2 RL rule table against fixtures authored in the same session as the rules.

**That was false about the W3C material.** Fetching
<https://www.w3.org/2009/11/owl-test/all.rdf> directly yields 489
`otest:TestCase` nodes, of which **206 are `otest:PositiveEntailmentTest`** and
**23 are `otest:NegativeEntailmentTest`**; 203 of the positives carry an RDF/XML
premise *and* an RDF/XML conclusion.

The statement was true only of the **flattened private copy** this tree was taken
from: that flattening extracted `otest:rdfXmlPremiseOntology` and discarded
`otest:rdfXmlConclusionOntology`, which is precisely the half a grader needs to
decide an entailment. A property of one repository's export was reported as a
property of the W3C suite.

Those tests are now vendored — from W3C directly, premise and conclusion — and
graded in the sibling tree `../w3c-owl2-rl/`. See its `PROVENANCE.md`.

The flattened source also carries six entailment-shaped cases of its own (two
positive, two negative, plus two divergence cases), but they are **self-authored
fixtures of the sibling project**, published under CC-BY-4.0 with an upstream of
`gmeow-self-authored` — not W3C material. Vendoring them under this tree's
`LicenseRef-W3C-Test-Suite` declaration would misstate their provenance and their
license, so they remain out of scope for a W3C OWL 2 corpus.

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
