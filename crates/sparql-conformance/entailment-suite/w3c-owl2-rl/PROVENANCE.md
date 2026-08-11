<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Vendored W3C OWL 2 entailment corpus

This tree vendors the **entailment** half of the W3C OWL 2 test suite: for each
case, the premise ontology *and* the conclusion (or non-conclusion) ontology.
It is consumed by the OWL 2 RL grader (`crates/sparql-conformance/src/owl2_rl.rs`,
driven by `crates/sparql-conformance/tests/owl2_rl_conformance.rs`), which
forward-materializes each premise under `purrdf_entail::Regime::OwlRl` and checks
whether the target graph maps into the closure.

It is the sibling of `../w3c-owl2/`, which vendors *consistency* cases and grades
them with the `OWL-Direct` tableau. The two trees grade different lanes of the
reasoner and neither substitutes for the other.

## Why this tree exists

Before it, nothing graded PurRDF's OWL 2 RL rule table against third-party
material: the rules and the fixtures that scored them were written by the same
change. `../w3c-owl2/PROVENANCE.md` justified that by asserting that the upstream
W3C material contains no entailment tests.

**That assertion was false**, and this tree is the correction. Fetching the exact
URL that document cites gives 489 `otest:TestCase` nodes, of which **206 are
`otest:PositiveEntailmentTest`** and **23 are `otest:NegativeEntailmentTest`**;
203 of the positives carry both an `otest:rdfXmlPremiseOntology` and an
`otest:rdfXmlConclusionOntology`. The claim was true only of the *pre-flattened
private copy* that tree was taken from, whose flattening extracted the premise
literal and discarded the conclusion literal — exactly the half an entailment
grade needs. The counts above are reproducible from the recipe at the bottom of
this file.

## Source

- Upstream: the **W3C OWL 2 test suite**, exported as a single RDF/XML manifest at
  <https://www.w3.org/2009/11/owl-test/all.rdf>.
- Fetched **directly from W3C** on **2026-07-29**. Not re-exported, not routed
  through any other repository.
- The suite is a static W3C archive under `/2009/11/owl-test/` with no upstream
  VCS revision to pin. What is pinned instead is the byte identity of the fetched
  manifest:

  | Property | Value |
  |----------|-------|
  | `Last-Modified` | `Wed, 18 Nov 2009 15:49:34 GMT` |
  | `ETag` | `W/"2f32df-478a7306a7f80"` |
  | size | 3 093 215 bytes |
  | SHA-256 | `5383f1ddf4cf2f03703a2f886f41d4e5bc375633a1cfa94a03254fd89330f8bb` |

## What was vendored, and by what criterion

| Bucket | Upstream | Vendored | Criterion |
|--------|---------:|---------:|-----------|
| Positive entailment, `otest:profile RL`, `otest:semantics RDF-BASED`, with an RDF/XML premise **and** conclusion | 27 | **27** | The RL rule table has a claim on these: W3C itself places the case inside the OWL 2 RL profile, under the RDF-Based semantics the rule table is defined for. This is the oracle's discriminating lane. |
| Negative entailment, RDF-BASED, with an RDF/XML premise **and** non-conclusion | 23 | **23** | *All* of them. A chase is sound, so deriving a published non-entailment is an unsoundness whatever profile the case carries — soundness is owed on every case, so nothing is filtered by profile here. |
| **Total** | | **50** | 100 RDF/XML documents, 124 KB |

One further document is vendored, under `imports/` rather than `cases/` — see
*Support documents* below.

Counts verified against the fetched manifest, not inherited. The audit that
prompted this work predicted 27 for the first row; the data agrees exactly.

### What was deliberately not vendored, and why

| Upstream | Cases | Why not |
|----------|------:|---------|
| Positive entailment, RDF-BASED, premise + conclusion, **not** `otest:profile RL` | 171 | These are DL/EL/QL/Full entailments. A sound-but-incomplete RL chase failing to derive one is not a finding — it is the fragment working as specified — so grading them would produce ~171 ledger entries that all say the same uninformative thing and would swamp the 27 that carry signal. **They are not silently dropped**: `census.tsv` names every one of them with `rl_corpus = outside-rl-profile`, and the harness tallies that column on every run. |
| Positive entailment, `DIRECT` semantics only | 5 | The OWL 2 RL rule table is defined over the RDF-Based semantics. A DIRECT-only case has no RDF-Based verdict to grade against. |
| Entailment tests with no `otest:rdfXmlPremiseOntology` | 3 | Functional-syntax-only cases. PurRDF's codecs do not read OWL functional syntax, so there is nothing to load. |
| Non-entailment tests (`ConsistencyTest` / `InconsistencyTest` only) | 260 | Satisfiability-shaped; they belong to `../w3c-owl2/` and are graded there. |

Every one of the 489 upstream cases appears in `census.tsv` in exactly one of
these dispositions, and `census_accounts_for_every_upstream_case` cross-checks the
census against both corpora's directory listings — so a case cannot be dropped
from a corpus while the census still claims it is graded, nor vendored while the
census claims it is not.

## Support documents (`imports/`)

An `otest:rdfXmlPremiseOntology` literal is ONE document. When a premise
`owl:imports` another ontology, the manifest carries no literal for that other
ontology at all — so the vendored premise is not the whole premise, and OWL 2
defines the imports closure of an ontology to BE the ontology for every semantic
purpose. Exactly one vendored case is in that position:

| Case | Imports |
|------|---------|
| `webont-imports-011` | `http://www.w3.org/2002/03owlt/imports/support011-A` |

That document is therefore vendored too, from its OWN W3C URL:

| Property | Value |
|----------|-------|
| path | `imports/support011-A.rdf` |
| source | <http://www.w3.org/2002/03owlt/imports/support011-A> |
| fetched | **2026-08-01**, directly from W3C |
| media type | `application/rdf+xml` |
| size | 885 bytes |
| SHA-256 | `f92a919635e21ad412662c5544a1a9003652a3c8a09ae25620fc2e29a72a2572` |
| `xml:base` | `http://www.w3.org/2002/03owlt/imports/support011-A`, declared in the document |

Reproduce with:

```sh
curl -sSL -o crates/sparql-conformance/entailment-suite/w3c-owl2-rl/imports/support011-A.rdf \
    http://www.w3.org/2002/03owlt/imports/support011-A
```

Three decisions in that table are load-bearing:

- **Outside `cases/`.** `census_accounts_for_every_upstream_case` requires every
  directory under `cases/` to carry a census row, and a support ontology is not a
  test case: it has no `otest:identifier`, no direction and no published verdict.
  Putting it under `cases/` would either break that cross-check or force a
  fabricated census row.
- **Keyed by the ontology IRI the document declares**, not by its file name.
  `owl2_rl::vendored_imports` reads the document's single named `owl:Ontology`
  subject and uses that as the key; a document with none, or with more than one, or
  with a blank-node one, is a hard error. A file-name convention would be a rule
  nothing enforces, and a re-vendor that renamed a file would silently stop
  resolving an import — the loudest possible failure turned into the quietest.
- **It declares its own `xml:base`**, so its relative `rdf:ID` references resolve
  against W3C's IRI rather than against the harness's synthetic one. That is
  checked, not assumed: `no_vendored_document_needs_the_harness_base` now sweeps
  **every** `.rdf` under this tree (`owl2_rl::vendored_documents`) rather than the
  two documents of each case, so a payload arriving in a new directory is covered
  the day it arrives.

Nothing about this makes the library fetch anything. `vendored_imports` builds a
`purrdf_entail::ImportMap` — caller-supplied configuration, exactly like every
other vocabulary this library reads — and hands it to `entails()`. A premise that
imports something the map does not resolve still refuses by name, and the
resolution is transitive to a fixpoint, so `support011-A`'s own imports would be
followed too.

## Fidelity — byte-exactness, stated precisely

| File | Fidelity |
|------|----------|
| `cases/<case>/premise.rdf` | The **exact value** of that case's `otest:rdfXmlPremiseOntology` literal, UTF-8 encoded. |
| `cases/<case>/conclusion.rdf` | The exact value of `otest:rdfXmlConclusionOntology`. |
| `cases/<case>/non-conclusion.rdf` | The exact value of `otest:rdfXmlNonConclusionOntology`. |
| `census.tsv` | **Derived**, mechanically, from the same manifest — see below. |

"Exact value" is the XML infoset value of the literal: reading the manifest at all
expands its character/entity references and normalizes CR-LF to LF, because that
is what XML parsing *is*, and nothing beyond that is done. In particular
**no trailing whitespace is stripped, no trailing newline is added, no namespace
is rewritten, no SPDX header is injected**, and nothing in this tree has been
hand-edited. Several vendored documents therefore end without a final newline,
and several carry trailing spaces — that is the W3C's byte stream, kept.

> This is a real difference from `../w3c-owl2/`, whose `PROVENANCE.md` claims its
> premises are "verbatim, byte-for-byte … no whitespace was normalized". They are
> not: re-extracting all 261 from this manifest reproduces them byte-for-byte only
> after stripping trailing whitespace from every line and appending a final
> newline (0/261 match raw; 261/261 match normalized). That document has been
> corrected.

Once the byte-freeze manifest covers this tree (see *Freeze* below), that claim is
enforced rather than merely stated.

## Base IRI

Each document is parsed by PurRDF's first-party RDF/XML codec under a synthetic,
deterministic base IRI (`http://example.org/w3c-owl2-rl/<case>` — an
`example.org` fixture IRI, per the repository's no-fabricated-vocabulary rule).
That base is never actually consulted. Of the 100 vendored documents, **95
declare their own `xml:base`** and the other **5 use only absolute IRIs** — the
`owl2-rl-rules-*` cases, whose every `rdf:about` / `rdf:resource` is written out
under `http://owl2.test/rules/`. Either way there is nothing for a base to
resolve, so no verdict depends on the harness's choice. The harness test
`no_vendored_document_needs_the_harness_base` checks both halves of that — it
scans every `rdf:about`, `rdf:resource`, `rdf:ID` and `rdf:datatype` of a
base-less document for a scheme — rather than taking it on trust.

## How a case is graded

1. Parse `premise.rdf` and the target document.
2. Hand both to `purrdf_entail::entails(&premise, &target, Regime::OwlRl,
   &imports)` — the library's conclusion-directed entailment service, which is
   what any caller gets, with `imports` the corpus's own `vendored_imports` map.
   The harness owns no reasoning of its own; it parses documents and compares one
   answer.
3. That call resolves `owl:imports` transitively against that map (an unresolved
   one is a refusal that NAMES the document), establishes the premise's
   **consistency** (an inconsistent premise entails everything, so it is a refusal
   and never a verdict), forward-materializes under `Regime::OwlRl`, and then
   reaches the conclusion one of six ways:
   - **matching** — does the target graph map into the closure, with its blank
     nodes read as existentials? A backtracking homomorphism with a candidate
     budget; exhausting the budget is a *withhold*, never a verdict.
   - **refutation** — for a conclusion whose shape no rule of Tables 4–9 has a
     head for, assert its negation into the premise and re-run the same rule
     table; the seventeen `false`-concluding rules are what decide it. Sound
     only because step 3 established the premise's consistency first.
   - **freeze-and-chase** — for a schema axiom abbreviating a Horn implication,
     instantiate its body over constants the premise does not mention, re-run the
     same table, and look for its head.
   - **comprehension** — for a conclusion asserting that an anonymous class
     exists, mint exactly the scaffolds it names, under the typing side conditions
     the RDF-Based comprehension conditions impose.
   - **reflexivity** — for a self-loop `x p x` over a property the premise
     declares reflexive, read it off the semantic condition.
   - **datatype containment** — for an `rdfs:range` over a datatype, intersect the
     premise's declared ranges and ask whether the intersection is contained in
     the conclusion's.

   None of the five beyond matching adds a rule, and each carries its own warrant
   arm and its own checker that re-decides the claim without running a reasoner.
4. Compare with the published direction. A positive case passes when the service
   answers `Entailed`; a negative case passes when it does not.

Three buckets, never two — **agree**, **withhold** (a refusal: a parse failure, a
chase error, a budget exhaustion), **disagree** — and every withhold and
disagreement must carry a typed `RlGap` in `owl2_rl::LEDGER`.

### What the two lanes can and cannot prove

The positive lane is one-sided in the honest direction: because the chase is
sound, a match *proves* the entailment. The negative lane is one-sided the other
way: a match is a proven **unsoundness**, while a non-match is the expected
answer and proves little on its own — a reasoner that derived nothing would pass
all 23. The discrimination therefore lives in the positive lane, where a reasoner
that derived nothing would fail all 27. The scoreboard reports both counts
separately for exactly this reason, and it also reports how many ledgered
divergences are *actionable* (a sound rule inside RL's own rule shape that the
table omits) as opposed to descriptions of what the profile cannot reach.

## What the oracle measured

```
OWL2-RL-ENTAILMENT: agreed 50 ledgered 0 unledgered 0 stale 0 total 50 actionable 0
```

```
OWL2-RL-NEGATIVE: total 23 = refuted 3 + admitted 20 (premise-outside-rl 5, conclusion-outside-rl 10, construct-not-read 5, refutation-budget 0, freeze-budget 0, data-range-containment 0) + unsound 0 + withheld 0
```

- **Negative lane: 23 / 23 agree. No unsoundness was found** — nothing W3C
  publishes as not entailed was ever reached. The second line above says what
  those 23 agreements are made of, because they are not one kind of result:
  **3 are decided refutations** (`new-feature-keys-004`, `webont-imports-002`,
  `webont-miscellaneous-301` — the premise is inside the RL syntax *and* the
  non-conclusion is an assertional graph over named terms, so both halves of
  Theorem PR1's hypothesis hold and the absence of a match is a proof), and
  **20 are named admissions** (the observation was made, nothing beyond it is
  claimed, and the missing entitlement is named). Every one of the 23 makes the
  soundness observation this lane grades — which is why every one of them
  agrees, and why the "no unsoundness" claim above is unqualified. What only 3
  of them carry is the entitlement to call it a refutation.
- **Positive lane: 27 of 27 agree.** The ledger is EMPTY.

That number moved 33 → 34 → 42 → 50 across five changes, and **the rule table did
not change once**. `rules(Regime::OwlRl)` and `implemented(Regime::OwlRl)` are the
same 78 they were, `extensions(Regime::OwlRl)` is the one `ext-eq-diff-sym`, and
strict `Materialization::OwlRl` output is byte-for-byte what it was. What changed
is how many times the table is run, what it is run over, and what a run's `false`
is read as — plus, once, which documents the premise consisted of.

The classes the ledger used to hold, and what closed each:

| Was | Cases | Closed by |
|-----|------:|-----------|
| `missing-rule` | 1 | a DECLARED extension, `ext-eq-diff-sym`, named in `extensions()` and in every report and in neither `rules()` nor `implemented()` |
| `negative-conclusion` | 6 | **refutation** — assert the conclusion's negation, re-run the same table, read its own seventeen `false`-concluding rules as the proof |
| `schema-conclusion` | 8 | two by refutation (an `owl:AllDifferent` IS its pairwise inequalities), one by **freeze-and-chase**, two by **comprehension**, three by **datatype containment** |
| `construct-outside-rl` | 1 | **reflexivity** — established positively from the semantic condition, which needs no completeness theorem and therefore no profile membership |
| `imports-unresolved` | 1 | vendoring the document the premise names, above |

Details of the first three are below; the mechanisms themselves are documented in
`purrdf-entail`'s `entails::{refutation, freeze, comprehension, reflexivity,
datarange}` modules, each with its soundness argument written out.

### The eight that were structural in ONE reading and not in the other

The ledger used to hold six `negative-conclusion` entries and two more filed as
`schema-conclusion`, and the reason given for all eight was true as far as it
went: no head anywhere in Tables 4–9 is a negative fact, so no forward chase over
those rules derives an `owl:differentFrom`, a membership in an anonymous
`owl:complementOf` class, or an `owl:AllDifferent` collection. What that reading
missed is that **seventeen of the seventy-eight rules conclude `false`**, and
seventeen rules that conclude `false` are an inconsistency calculus. A negative
fact does not need a rule with a negative head; it needs a refutation.

`purrdf_entail::entails()` performs one. It asserts the conclusion's negation
into the premise — `owl:sameAs` for an `owl:differentFrom`, a class assertion for
an `owl:complementOf` membership — and re-runs the same rule table over a premise
whose consistency the first run already established. Across the eight cases the
rule that actually reaches `false` is `cax-dw`, `cax-adc`, `prp-pdw`, `prp-adp`
or `eq-diff1` — measured, not guessed. `new-feature-objectqcr-002` is the longest
chain of the eight: the asserted `Stewie a Woman` lets `cls-maxqc3` derive
`Stewie owl:sameAs Meg` against a `maxQualifiedCardinality 1`, and `eq-diff1`
then clashes that against the premise's own `Stewie owl:differentFrom Meg`. An
`owl:AllDifferent` collection is, by OWL 2's own definition, the conjunction of
its `n(n−1)/2` pairwise inequalities, so it lowers to the same shape and is
entailed exactly when every pair refutes — which is why two `schema-conclusion`
entries left with the six.

Nothing was added to the table to do it: `rules(Regime::OwlRl)` and
`implemented(Regime::OwlRl)` are still exactly the same 78, and
`extensions(Regime::OwlRl)` is still the one rule named below. This row moved
34 → 42 and the ledger 16 → 8 on a second run of the same seventy-eight rules.

### The eight that remained, and the four mechanisms that closed them

`chain2trans1` concludes `p rdf:type owl:TransitiveProperty` from
`p owl:propertyChainAxiom (p p)`. Still no head in Tables 4–9 — but the axiom
ABBREVIATES a universally quantified Horn implication, and an implication is
decided by **generalisation on constants**: freeze `_:a p _:b . _:b p _:c` over
constants the premise does not mention, re-run the same table, and `prp-spo2`
derives `_:a p _:c`. Deliberately not routed through the DL tableau, whose reverse
mapping DROPS `owl:propertyChainAxiom` and would therefore answer a confident
wrong "not entailed".

`webont-i5-5-005` and `webont-i5-26-010` conclude an anonymous `owl:unionOf` class
and an anonymous `owl:Restriction`. Neither says anything about any individual;
each says a class EXISTS, which the RDF-Based semantics' own **comprehension
conditions** license — subject to a typing side condition on the operands, which
is why `i5-5-005`'s premise `a rdf:type owl:Class` is the whole difference between
a published entailment and a published non-entailment. Only the scaffolds the
conclusion names are minted, over blank nodes checked absent from both documents.

`new-feature-reflexiveproperty-001` concludes `Peter knows Peter` from
`knows a owl:ReflexiveProperty`. `owl:ReflexiveProperty` is outside the RL syntax
so no rule fires — and a rule that DID fire would range over every resource,
widening a closure every consumer computes by default. It is established
**positively** instead, from the semantic condition: a reflexive property holds of
every element of `IR`, and every IRI denotes one.

`webont-i5-8-006`, `-008` and `-009` conclude an `rdfs:range` **widened** to a
containing XSD datatype. Widened, not narrowed — `xsd:byte ⊑ xsd:short` — which is
why they are sound at all; two of the three need the INTERSECTION of several
declared ranges. Deciding that needs the XSD value spaces, so it is decided by
`purrdf_xsd::range::containment`, three-valued, with the negative answer gated on
the counterexample range being exactly decided.

### The one that was NOT structural, and how it was closed

`webont-differentfrom-001` is `a owl:differentFrom b` ⊨ `b owl:differentFrom a` —
plain symmetry, a positive assertional head over two named individuals, shaped
exactly like `prp-symp`, and sound to state. It is **not one of the 78 rules of
Tables 4–9**: Table 4's `owl:differentFrom` rules only conclude `false`. So a
rule set can be complete for the table and still stop one triple short of a
W3C-published entailment, which is exactly what an independent oracle is for.

PurRDF states it, and states it as PurRDF's rather than as W3C's.
`purrdf_entail::RuleId::ExtEqDiffSym` (`ext-eq-diff-sym`) lives in a rule family
declared to sit outside every specification table: `extensions(Regime::OwlRl)`
names it, `rules(Regime::OwlRl)` and `implemented(Regime::OwlRl)` are both still
exactly the same 78 and name none of it, `RuleId::is_extension` decides which is
which, and every rendered report carries an `extension ext-eq-diff-sym` line
beside its `missing` lines. So this row moved 33 → 34 — the change before the
eight above — and the ledger's `actionable` count 1 → 0, while `OWL-RL 78 / 78`
still means Tables 4–9 and nothing else.

## The upstream census (`census.tsv`)

One row per upstream `otest:TestCase` — 489 rows, tab-separated, sorted by case
slug. Columns:

| Column | Meaning |
|--------|---------|
| `identifier` | the upstream `otest:identifier`, verbatim |
| `case` | that identifier slugified to a directory name (`lower()`, every run of non-`[a-z0-9]` to `-`) |
| `otest_types` | the `rdf:type`s the case declares, `;`-joined, local names only |
| `semantics`, `profiles`, `status`, `normative_syntax` | the corresponding `otest:` values |
| `premise` | whether an `otest:rdfXmlPremiseOntology` is present |
| `conclusion` | `conclusion`, `non-conclusion`, or `none` |
| `rl_corpus` | the disposition table above |
| `dl_corpus` | `graded` / `not-vendored` / `not-a-consistency-test` for `../w3c-owl2/` |
| `dl_probe` | for a consistency-shaped case `../w3c-owl2/` does **not** vendor: what the tableau actually did with it |

`dl_probe` is a **measurement**, not an inherited claim. Every one of the 198
excluded consistency-shaped cases that carries an RDF/XML premise was run through
`purrdf_entail::materialize_dl_reported` on **2026-08-10**, in a release build, one process
per case, four at a time, with a **40 s wall-clock ceiling** per case:

| `dl_probe` | Cases | Meaning |
|------------|------:|---------|
| `decides-consistent` | 109 | the tableau terminates and answers |
| `decides-inconsistent` | 64 | likewise |
| `non-terminating` | 0 | killed at the 40 s ceiling — the reasoner cannot decide these |
| `withholds-reasoner` | 20 | an honest `EntailError`, or a `budget-exhausted`/boundary-caveated answer (a step cap, an unread cardinality/list shape, a nominal/inverse/counting boundary) |
| `withholds-parse` | 5 | the RDF/XML codec refuses (a DTD, an `rdf:datatype` with node content) |
| `no-rdfxml-premise` | 23 | functional-syntax-only; nothing to load |

The prior measurement (2026-07-29, debug build) found 30 non-terminating cases,
including `webont-i5-8-001`. The whole-TBox clausification and search-refinement
work this reasoner underwent since then made every one of those 30 terminate
within the same ceiling: 17 now decide outright, 13 now reach the search's own
budget and answer `budget-exhausted` (a `withholds-reasoner` disposition, not a
capability the harness's wall clock cuts short). Non-termination is therefore
**empty** at this ceiling for the first time this file has recorded a
measurement; the `owl2_conformance` harness's non-terminating constant moved from
30 to 0 to match.

**Two decided verdicts changed direction against the prior measurement**, and
both are named here rather than folded silently into the tally above:

- `datatype-float-discrete-001` moved `decides-consistent` → `decides-inconsistent`.
  The published verdict is `InconsistencyTest`, so the prior measurement had this
  one wrong; the new run decides it in 2 rounds with `completeness decided` (no
  boundary), and is now correct. Its premise types an individual by an
  `xsd:float` open interval `(0.0, 1.401298464324817e-45)` — the smallest
  positive `float` — whose value space a discrete datatype makes empty, so the
  ontology is unsatisfiable; the prior search evidently never reached that
  data-range check.
- `webont-description-logic-035` moved `decides-inconsistent` → `decides-consistent`,
  the one direction this file's own doctrine treats as suspect rather than a
  quiet win. The published verdict is `InconsistencyTest`, so the new answer
  disagrees with W3C — but it is rendered `completeness decided-within-boundaries`
  with a `boundary counting-on-inverse`: a nominal ("spy point") bounded by an
  `owl:maxCardinality` over an *inverse* role, textbook of the one completeness
  gap this reasoner already discloses by name (the missing NN/NI
  nominal-introduction rule; see `Construct::CountingOnInverse` in
  `crates/entail/src/report.rs`). The prior measurement, on a reasoner that took
  a different search path through the same axioms, apparently found the clash
  without ever exercising that gap; the current path does not, and says so
  rather than guessing. This is a genuine, pre-existing incompleteness corner
  surfacing on a case it previously missed, not a new defect — and it is why
  this row is not silently absorbed into the tally.

The other headline the probe produced: of the 221 consistency-shaped cases the DL
corpus leaves out, **173 the tableau decided when probed** (up from 156). Their
exclusion is a payload-size and triage decision, not a capability limit, and
reporting "257 agreed of 261" without that context overstates the coverage. That
is why the harness prints `OWL2-DL-EXCLUDED` next to `OWL2-ENTAILMENT`.

## License

The W3C OWL 2 test files are published under the **W3C Test Suite License** /
**W3C Software and Document License** — see
<https://www.w3.org/Consortium/Legal/2015/copyright-software-and-document>. They
are vendored verbatim and are **not** relicensed. Rather than 101 per-file
`.license` sidecars, the tree declares them with a single `REUSE.toml` glob
annotation naming `LicenseRef-W3C-Test-Suite` — *not* the bare SPDX id `W3C`,
which denotes the W3C Software Notice and License, a different document. The
license text is in `LICENSES/LicenseRef-W3C-Test-Suite.txt`. This document and the
grader that reads the tree are PurRDF-authored (MIT OR Apache-2.0).

## Freeze

This tree is vendored payload and is byte-frozen like every other, `imports/`
included. It has one entry in `scripts/check-corpus-frozen.py`'s `GUARDED_ROOTS`:

```python
"crates/sparql-conformance/entailment-suite/w3c-owl2-rl": (
    "scripts/conformance-frozen/sparql-conformance-w3c-owl2-rl.sha256"
),
```

`python3 scripts/check-corpus-frozen.py --update` writes the manifest.
(`PROVENANCE.md`, `REUSE.toml` and `LICENSES/` are skipped by the freeze gate, so
editing this document never requires regenerating it.)

## Re-vendoring

Everything below is self-contained: it needs only `curl` and Python's standard
library, and it reproduces this tree bit-for-bit from the W3C URL.

```sh
curl -sSL https://www.w3.org/2009/11/owl-test/all.rdf -o /tmp/all.rdf
python3 revendor.py /tmp/all.rdf crates/sparql-conformance/entailment-suite/w3c-owl2-rl
```

```python
# revendor.py — reproduces cases/ from the W3C manifest.
import os, re, sys, xml.etree.ElementTree as ET

RDF = "{http://www.w3.org/1999/02/22-rdf-syntax-ns#}"
T = "{http://www.w3.org/2007/OWL/testOntology#}"
manifest, out = sys.argv[1], sys.argv[2]

cases = [c for c in ET.parse(manifest).getroot() if c.tag == T + "TestCase"]
types = lambda c: {t.get(RDF + "resource").rsplit("#", 1)[1] for t in c.findall(RDF + "type")}
res = lambda c, k: {e.get(RDF + "resource").rsplit("#", 1)[1]
                    for e in c.findall(T + k) if e.get(RDF + "resource")}
txt = lambda c, k: (c.find(T + k).text if c.find(T + k) is not None else None)
slug = lambda i: re.sub(r"[^a-z0-9]+", "-", i.lower()).strip("-")

def write(path, text):                      # byte-exact: no rstrip, no added newline
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, "wb").write(text.encode("utf-8"))

for c in cases:
    ty, sem, prof = types(c), res(c, "semantics"), res(c, "profile")
    prem = txt(c, "rdfXmlPremiseOntology")
    con = txt(c, "rdfXmlConclusionOntology")
    non = txt(c, "rdfXmlNonConclusionOntology")
    s = slug(txt(c, "identifier"))
    if "PositiveEntailmentTest" in ty and "RL" in prof and "RDF-BASED" in sem and prem and con:
        write(f"{out}/cases/{s}/premise.rdf", prem)
        write(f"{out}/cases/{s}/conclusion.rdf", con)
    elif "NegativeEntailmentTest" in ty and "RDF-BASED" in sem and prem and non:
        write(f"{out}/cases/{s}/premise.rdf", prem)
        write(f"{out}/cases/{s}/non-conclusion.rdf", non)
```

Then:

1. Rebuild `census.tsv`'s W3C-derived columns from the same manifest, re-measure
   the `dl_probe` column (recording the new date and ceiling in the table above),
   and update the *What was vendored* counts to whatever the data says.
2. Regenerate the freeze manifest:

   ```sh
   python3 scripts/check-corpus-frozen.py --update
   ```

3. Re-measure the ledger:

   ```sh
   cargo test -p purrdf-sparql-conformance --locked --test owl2_rl_conformance -- \
       --ignored --nocapture regenerate_rl_ledger
   ```

   It prints a paste-ready replacement for `LEDGER` in
   `crates/sparql-conformance/src/owl2_rl.rs`, with a `RlGap::TypeMe` placeholder
   on every entry. Every one must be given a typed reason **by hand** — an
   unexplained divergence is not a ledger entry.
