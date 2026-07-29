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

1. Parse `premise.rdf`; forward-materialize it with
   `purrdf_entail::materialize(&ds, Regime::OwlRl)`.
2. Parse the target document into its default-graph triples.
3. Ask whether the closure **simple-entails** the target: does the target graph
   map into the closure, with the target's blank nodes read as existentials? The
   search is a backtracking homomorphism with a candidate budget; exhausting the
   budget is a *withhold*, never a verdict.
4. Compare with the published direction. A positive case passes when the closure
   contains the conclusion; a negative case passes when it does not.

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
OWL2-RL-ENTAILMENT: agreed 34 ledgered 16 unledgered 0 stale 0 total 50 actionable 0
```

- **Negative lane: 23 / 23 agree. No unsoundness was found** — the chase never
  derived a triple W3C publishes as not entailed.
- **Positive lane: 11 of 27 agree.** The other 16 are ledgered with typed reasons:
  8 `schema-conclusion`, 6 `negative-conclusion`, 1 `construct-outside-rl`,
  1 `imports-unresolved`.

The 16 are not 16 bugs. Every one of them is a structural property of the OWL 2
RL/RDF rule table rather than of this implementation: every head in Tables 4–9 is
an assertional triple over named terms or `false`, so no conforming RL rule set
derives a schema axiom (`p a owl:TransitiveProperty`, an `rdfs:range`, an
anonymous `owl:Restriction`, an `owl:AllDifferent`), and none derives a negative
fact (`owl:differentFrom`, membership in an `owl:complementOf`), which follows
only by refutation. W3C still tags those cases `otest:profile RL`, because that
tag describes the *ontology's* profile and not what the rule table reaches.

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
beside its `missing` lines. So this row moved 33 → 34 and the ledger's
`actionable` count 1 → 0, while `OWL-RL 78 / 78` still means Tables 4–9 and
nothing else.

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
`purrdf_entail::materialize_dl` on **2026-07-29**, in a debug build, one process
per case, four at a time, with a **40 s wall-clock ceiling** per case:

| `dl_probe` | Cases | Meaning |
|------------|------:|---------|
| `decides-consistent` | 93 | the tableau terminates and answers |
| `decides-inconsistent` | 63 | likewise |
| `non-terminating` | 30 | killed at the 40 s ceiling — the reasoner cannot decide these |
| `withholds-reasoner` | 7 | an honest `EntailError` (a step cap, an unread cardinality/list shape) |
| `withholds-parse` | 5 | the RDF/XML codec refuses (a DTD, an `rdf:datatype` with node content) |
| `no-rdfxml-premise` | 23 | functional-syntax-only; nothing to load |

`../w3c-owl2/PROVENANCE.md` said 32 cases were excluded for non-termination. This
measurement finds **30** at a 40 s ceiling, and names them — the number is
budget-dependent, so it is recorded here with its budget rather than quoted
loose. `webont-i5-8-001`, the one case that document named, is among them. The
`owl2_conformance` harness pins both 221 and 30 as constants and prints the 30 by
name, so the exclusion cannot go quiet again.

The other headline the probe produces: of the 221 consistency-shaped cases the DL
corpus leaves out, **156 are decided today**. Their exclusion is a payload-size
and triage decision, not a capability limit, and reporting "256 agreed of 261"
without that context overstates the coverage. That is why the harness prints
`OWL2-DL-EXCLUDED` next to `OWL2-ENTAILMENT`.

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

This tree is vendored payload and must be byte-frozen like every other. That
needs one entry in `scripts/check-corpus-frozen.py`'s `GUARDED_ROOTS`:

```python
"crates/sparql-conformance/entailment-suite/w3c-owl2-rl": (
    "scripts/conformance-frozen/sparql-conformance-w3c-owl2-rl.sha256"
),
```

then `python3 scripts/check-corpus-frozen.py --update` to write the manifest.
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
