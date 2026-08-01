<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Entailment

[`purrdf-entail`](https://docs.rs/purrdf-entail) (re-exported as
`purrdf::entail`) is native, `wasm32`-clean entailment for the PurRDF
`RdfDataset` IR. A family of engines sits behind one facade, each the right
tool for its SPARQL entailment regime — closing a dataset to its inferred
fixpoint entirely in interned `TermId` space, with **no** external reasoner,
no async runtime, and no string round-trip.

## Surface map

| Entry point | Regime(s) | Engine |
| --- | --- | --- |
| `materialize(ds, regime)` | `Simple`, `RDF`, `RDFS`, `OWL-RL`, `D` | Forward materialization ("chase") of the regime's declared clause program via a native semi-naive fixpoint. Returns `(closure, ReasoningReport)`; the report is not optional. |
| `materialize_dl_reported(...)`, or `materialize(ds, Materialization::OwlDirect(bgp))` | `OWL-Direct` | Open-world OWL DL over a SHOIQ(D) tableau, directed by the query's basic graph pattern `bgp`; `materialize` delegates to it for this regime rather than restating it. |
| `materialize_rif(...)` | `RIF` | RIF-Core rule entailment over a parsed `RuleSet`. |
| `parse_rif_xml(...)` / `resolve_rif_imports(...)` | `RIF` | RIF-XML parsing with caller-owned, I/O-free import resolution. |
| `rules(regime)` / `implemented(regime)` | — | The rule table a regime is *defined by*, and the subset this workspace fires. Their difference is the measurable gap. |
| `calculus_program(regime)` | — | The regime's calculus as DL-clause data — the very program `materialize` evaluates, so a consumer can recompute its contract hash. |
| `Regime::from_iri(iri)` | — | Parse a `sparql:entailmentRegime` IRI to its enum. |

```rust,ignore
use purrdf::entail::{materialize, Completeness, Materialization};

// Close a frozen dataset to its RDFS fixpoint; the result is a new dataset
// AND a report of what the run did.
let (closed, report) = materialize(&ds, Materialization::Rdfs).expect("materializes");
assert_eq!(report.completeness(), Completeness::ExactWithinBoundaries);
```

## The same engine in four hosts

Entailment is not re-implemented per host. Python, WebAssembly, and the C ABI all
route through one shared string boundary (`purrdf_validate::regime`) that wraps
the Rust engine, and all four surfaces are checked against a single committed
golden-vector artifact — so a divergence shows up as one vector failing rather
than as three surfaces that quietly stopped agreeing. The regime spellings
(`simple`, `rdf`, `rdfs`, `owl-rl`, `owl-direct`, `rif`, `d`) are the same
everywhere.

| Host | Materialize | Defined rule table | Implemented rules |
| --- | --- | --- | --- |
| Rust | `materialize(&ds, Materialization::Rdfs)` | `rules(Regime::Rdfs)` | `implemented(Regime::Rdfs)` |
| CLI | `purrdf reason --regime rdfs`, `purrdf convert --entailment rdfs`, `purrdf query --entailment rdfs` | — | — |
| Python | `purrdf.entail.materialize(dataset, "rdfs", "")`, `purrdf.entail.materialize_nt(text, "rdfs", "")` | `purrdf.entail.rules("rdfs")` | `purrdf.entail.implemented_rules("rdfs")` |
| JavaScript / WebAssembly | `entailMaterialize(doc, "rdfs", "")` | `entailRules("rdfs")` | `entailImplementedRules("rdfs")` |
| C | `purrdf_entail_materialize_to_nquads(...)` | `purrdf_entail_rules(...)` | `purrdf_entail_implemented_rules(...)` |

Every host materializes every regime; none refuses one. What two regimes need is an
INPUT, and each host has a parameter for it: `--rules <FILE>` on the CLI, a
`program` string on the Python, WebAssembly and C surfaces, and the
`Materialization` value itself in Rust. `rif` takes a normative RIF-in-XML rule
document there; every other regime takes none, and supplying one is an error rather
than a discarded argument. `owl-direct`'s extra input is a *query's* class
expressions, so a document-in/document-out call runs the query-independent
augmentation and a query surface (`purrdf::query_with_entailment`,
`purrdf query --entailment owl-direct`) is where the query-directed lane lives.

One host-specific note:

- The **WebAssembly** module also exports `entailCheckGoldenVectors()`, which
  replays the committed tri-host vector artifact inside the module a consumer
  actually loaded — so agreement with the reference implementation can be checked
  without trusting this repository's CI.

## Rule coverage

`rules(regime)` is the rule table the specification defines the regime by;
`implemented(regime)` is the subset the evaluator fires. Both are `&'static`
slices in specification table order, so the gap is an executable artifact instead
of a sentence:

| Regime | Rule table | Defined | Implemented |
| --- | --- | ---: | ---: |
| `Simple` | — (identity closure) | 0 | 0 |
| `RDF` | RDF 1.2 Semantics §8.1.1 | 3 | 3 |
| `RDFS` | RDF 1.2 Semantics §8.1.1 + §9.2.1 | 18 | 18 |
| `OWL-RL` | OWL 2 Profiles §4.3 Tables 4–9 | 78 | 78 |
| `D` | OWL 2 Profiles §4.3 Table 8 | 5 | 5 |
| `OWL-Direct` | — (SHOIQ(D) tableau, not a fixed table) | 0 | 0 |
| `RIF` | — (caller-supplied rule set) | 0 | 0 |

The per-rule breakdown — every rule id, its specification citation, and whether
it is fired — is [generated from that API](entailment-rules.md) and
drift-guarded, so it cannot fall behind the code.

Where the numbers stop:

- **The four existential rules fire, but their conclusions are withheld.**
  `rdfD1`, `rdfD1a`, `rdfs14` and `rdfs14a` each conclude about a *fresh* blank
  node. The restricted chase mints each one as a frontier-addressed Skolem
  witness and closes under it, so the rules genuinely fire — but every
  conclusion mentioning a surrogate is dropped when the closure is materialized
  back, because a SPARQL entailment regime draws its answers from the scoping
  graph and a surrogate is not in it. The withholding is reported as
  `Construct::Surrogate`. Nothing surrogate-free is lost: replacing a term with
  a fresh blank node only weakens a triple.
- **A complete rule table is not a complete closure.** `OWL-RL` fires all 78
  rules, and a run that met a boundary still reports
  `Completeness::ExactWithinBoundaries` rather than `Exact`. The two claims are
  reported separately on purpose. Nor is a complete rule table entailment
  conformance: on W3C's own OWL 2 RL entailment tests `entails()` reaches 27 of
  27 published positive entailments and correctly withholds on 23 of 23
  negative ones (see [Conformance](#conformance) below). 78 / 78 says every
  rule of Tables 4–9 is implemented — and the one W3C-published entailment
  that is reachable only by a sound rule outside those tables is reached by an
  **extension**, `ext-eq-diff-sym`, which `extensions(Regime::OwlRl)` names,
  neither `rules()` nor `implemented()` names, and every report renders on its
  own `extension` line. Eight more are reached by **refutation** rather than by
  matching, and those add no rule at all: see the conformance section below.
- **Seventeen OWL 2 RL rules conclude `false`.** "Implemented" for those means
  *decided*: a body match becomes `EntailError::Inconsistent` carrying a witness
  that names the rule and the asserted triples that satisfied it. That is the
  only thing a rule with no conclusion can do.

## The chase (Simple / RDF / RDFS / OWL-RL / D)

`materialize` runs a forward-materialization chase: a fixed rule set for the
selected regime, applied by a semi-naive fixpoint until no new quads appear.
Because it runs over the frozen IR, it is deterministic — a given input and
regime always yields the same closure — and because it works in `TermId`
space, no term is ever re-parsed or re-serialized along the way.

Typical use: materialize first, then query with the plain
[SPARQL engine](sparql/querying.md) or validate the closure with
[SHACL](validation/shacl.md) (the SHACL validator itself performs no
inference).

The rule set is not written twice. `calculus_program(regime)` renders it as
DL clauses and `materialize` evaluates exactly those clauses through
[`purrdf-datalog`](datalog.md)'s semi-naive evaluator, so the contract hash a
report carries identifies the clauses that actually ran.

## Every run says what it did

`materialize` returns `(closure, ReasoningReport)`. There is deliberately no
report-free variant, because the alternative — two entry points, one of which
discards the evidence — is how a partial rule set comes to be described as a
complete one. The report carries:

- **`Completeness`** — derived from `rules(regime)` minus `implemented(regime)`,
  so it improves by itself as rules are added, and it names the `missing` rules
  rather than merely counting them;
- **per-rule firing counts** — which rules fired and how many conclusions each
  contributed;
- **`Boundary`s** — the constructs the run met and could not close over, each
  with its reason;
- **the evaluation budget** — what the run consumed of the evaluator's fixed
  ceilings;
- **a contract hash** — `purrdf-datalog`'s digest of the clause program, so a
  cached closure minted under a different calculus can be *refused* rather than
  trusted;
- **an inconsistency witness**, when a rule that concludes `false` matched: the
  rule id, the asserted triples that satisfied its premises in premise order, and
  the graph they were read from.

A report cannot claim `Exact` while naming a boundary. `ReasoningReport` stores
no completeness field at all: `completeness()` derives the value from the
boundary list itself, so the contradictory state is unrepresentable rather than
merely checked. That is deliberate — an earlier design stored the field and
compared it against a derivation of the same inputs, which is vacuous by
construction and could never fail.

The rendering is byte-stable, so the Python, WebAssembly, and C hosts hand back
the same report text as Rust for the same input.

## OWL-Direct: the tableau

`OWL-Direct` semantics is open-world Description Logic, which a forward chase
cannot answer. `materialize_dl_reported` runs an **SHOIQ(D) tableau** instead —
answering instance and subsumption queries via classification, realization,
and query-directed materialization. Because it needs the query's class
expressions, it takes them as its own `query_bgp` parameter; `materialize`
reaches the same tableau by delegating to it for `Materialization::OwlDirect`
rather than restating it.

## RIF

`materialize_rif` evaluates **RIF-Core** rules over a parsed `RuleSet`,
covering the SPARQL RIF entailment regime.

## D (datatype) entailment

`D` is materialized, not refused. PurRDF realizes it as Simple entailment plus
the five `dt-*` rules of OWL 2 Profiles §4.3 Table 8 — the fixed rule table a
forward chase can enumerate for it — decided over the XSD *value* space by
`purrdf-xsd` rather than by comparing lexical forms.

What Table 8 does not cover is the infinite value spaces themselves, and that is
reported as a `Construct::DatatypeValueSpace` boundary on the run rather than
claimed. So a `D` closure is complete *within its stated boundary*, and the report
is where the boundary is stated.

Every host materializes `d`, the command-line tool included.

There is **no unsupported-regime error**. `materialize` takes a `Materialization`,
which carries each regime's own input — a basic graph pattern for `OWL-Direct`, a
`RuleSet` for `RIF` — so all seven inhabitants of that type are served and a caller
cannot hand the function a value it accepts and get a refusal instead of an answer.
`Regime` stays as the reporting and identity type that `ReasoningReport::regime()`,
`rules()`, `implemented()` and `Regime::from_iri` speak in.

## Invariants

- **No minted vocabulary.** Every constant in the crate's `vocab` module is a
  standard `rdf:`/`rdfs:`/`owl:` IRI drawn from the entailment specs
  themselves — the crate fabricates none, per the
  [toolkit-not-ontology rule](project/design-rules.md).
- **Dependency-lean and wasm-clean.** The dependencies are `purrdf-core`,
  [`purrdf-datalog`](datalog.md), `purrdf-xsd`, `roxmltree`, `blake3`, and two
  fixed-key hashers (`ahash`, `hashbrown`) — every one of them
  `wasm32-unknown-unknown`-clean, so the engines carry into Rust, Python,
  WebAssembly, and C unchanged, with no threads, filesystem, or RNG dependency.
- **Deterministic.** Same input + regime → same closure, always — and the same
  report, byte for byte.

## Conformance

Two corpora measure two different things, and the distinction matters:

- **W3C SPARQL 1.1 entailment-regime group — 70 of 70 cases pass**, with zero
  ledgered residuals: the RDF/RDFS/OWL-RL chase, the OWL-Direct (DL) tableau, the
  RIF-Core rule engine, and RDF-axiomatic predicate typing, all run through the
  SPARQL conformance harness.
- **W3C OWL 2 test suite — 257 of 261 cases agree, 4 ledgered**, zero
  unledgered. This corpus is *consistency*-shaped: all 261 vendored cases are
  `otest:ConsistencyTest` (226) or `otest:InconsistencyTest` (35). It therefore
  grades the DL/tableau lane's satisfiability verdicts and says nothing about
  the OWL 2 RL rule table. Every one of the 4 divergences is named in a typed
  ledger; an unledgered divergence, and a ledgered case that has started
  agreeing, are both hard failures.

  Two things this row does **not** say. First, the upstream material is not
  free of entailment tests — the W3C manifest holds **206 positive and 23
  negative entailment tests**; this corpus lacks them because the flattening it
  was taken from extracted the premise literal and discarded the conclusion
  literal, which is exactly the half an entailment grade needs. They are
  vendored and graded by the next bullet. Second, the corpus is a **subset**:
  261 of the 482 consistency-shaped cases upstream. Of the 221 it leaves out,
  **156 the tableau decided when the exclusion was measured** (93 consistent, 63
  inconsistent), 30 did not terminate under a 40 s ceiling, 12 were withheld (7
  reasoner, 5 parse), and 23 carry no RDF/XML premise — so the exclusion was
  payload triage, not a capability limit, and "257 of 261" is a number over a
  corpus rather than over what W3C published.

  Those five figures are a **dated measurement**, recorded in `census.tsv`'s
  `dl_probe` column and described in that suite's `PROVENANCE.md`. The harness
  reads the column and prints it on every run; it does NOT re-run the reasoner
  over the 221 excluded cases, so this row cannot detect a regression among them.
  Re-deriving them means re-running the probe, which is a deliberate act rather
  than part of the gate.
- **W3C OWL 2 RL entailment tests — 50 of 50 cases agree, 0 ledgered**, zero
  unledgered. This is the independent oracle for the rule table: W3C's own
  entailment tests, answered by one call to `purrdf_entail::entails()` per case
  under `Regime::OwlRl`. The two lanes prove different things and are reported
  separately.

  **The negative lane is 23 of 23: no unsoundness.** The chase never derived a
  triple W3C publishes as *not* entailed. That is the safety result, and it
  holds over *all* 23 negative cases — soundness is owed on every case, so none
  were filtered by profile.

  The positive lane is **27 of 27** — the 27 positive entailments W3C itself
  places inside the RL profile under RDF-Based semantics — and the typed
  divergence ledger `purrdf_sparql_conformance::owl2_rl::LEDGER`
  is EMPTY — 0 `schema-conclusion`, 0 `negative-conclusion`, 0
  `construct-outside-rl`, 0 `imports-unresolved`, and **0 are actionable** (0
  `missing-rule`).

  Every class it used to hold is closed, and the rule table did not change once
  to close any of them. `entails()` reaches a conclusion six ways, and five of
  the six are not matching:

  * **refutation.** A negative fact still has no head anywhere in Tables 4–9 —
    no rule concludes `owl:differentFrom`, and none concludes membership in an
    `owl:complementOf` class. What the table *does* have is seventeen rules
    whose conclusion is `false`, and those seventeen are an inconsistency
    calculus: assert the conclusion's negation into the premise, re-run the same
    seventy-eight rules over a premise whose consistency the first run already
    established, and read the resulting inconsistency as the proof. An
    `owl:AllDifferent` collection is, by OWL 2's own definition, the conjunction
    of its `n(n−1)/2` pairwise inequalities, so it lowers to the same shape and
    is entailed exactly when every pair refutes — which is why two entries left
    the `schema-conclusion` class with them.
  * **freeze-and-chase.** `p rdf:type owl:TransitiveProperty` abbreviates a
    universally quantified Horn implication, and an implication is decided by
    generalisation on constants: freeze its body over constants the premise does
    not mention, re-run the table, and look for the head. `chain2trans1`'s
    arrives through `prp-spo2`, one of the 78. The axiom's other conjunct — `p`
    is an object property — is a lookup in the premise's own closure, and it is
    owed: a schema axiom is a conjunction and establishing only the interesting
    half would claim conclusions the semantics does not license.
  * **comprehension.** A conclusion may assert that a CLASS EXISTS — an
    anonymous `owl:unionOf`, an anonymous `owl:Restriction` — which the
    RDF-Based semantics' own comprehension conditions license, subject to a
    typing side condition on the operands. Only the scaffolds the conclusion
    names are minted, over blank nodes checked absent from both documents.
  * **reflexivity.** `owl:ReflexiveProperty` is outside the RL syntax, so the
    profile states no rule for it — and a rule that did would range over every
    resource, widening a closure every consumer computes by default. The
    conclusion's own self-loops are read off the premise's reflexive typings
    instead.
  * **datatype containment.** A property's declared `rdfs:range` datatypes
    intersect, and the intersection may be contained in one the premise never
    mentions — `xsd:byte ⊑ xsd:short`, and `short ⊓ unsignedInt ⊑
    unsignedShort`, neither of which a join over triples can discover. Decided
    over the XSD value spaces, three-valued, with the negative answer gated on
    the counterexample range being exactly decided.

  The last case needed no mechanism at all, only the document its premise names:
  `webont-imports-011` `owl:imports` a support ontology the upstream manifest
  does not inline, so it is vendored beside the cases from W3C's own URL and
  supplied to `entails()` as caller-owned configuration. The library still
  fetches nothing.

  Nothing about the inventory moves: `rules(Regime::OwlRl)` and
  `implemented(Regime::OwlRl)` are still exactly the same 78,
  `extensions(Regime::OwlRl)` is still the one `ext-eq-diff-sym`, and strict
  `Materialization::OwlRl` output is byte-for-byte what it was. The evidence
  moves instead — each mechanism arrives with its own `EntailmentWarrant` arm
  carrying what it actually used (the `false`-concluding rule that fired and a
  minimal entailing premise subset; the frozen constants, body and head; the
  minted triples and the closure triples that license them) and its own checker
  that re-decides the whole thing without running a reasoner.

  The one case that used to be actionable is closed by an **extension**, and
  the extension is labelled rather than absorbed. `a owl:differentFrom b`
  entails `b owl:differentFrom a`, which is sound — `owl:differentFrom` denotes
  inequality and inequality is symmetric — and shaped exactly like `prp-symp`,
  yet is not among the 78 rules, because Table 4's `owl:differentFrom` rules
  only ever conclude `false`. PurRDF states it as `ext-eq-diff-sym`, in a rule
  family declared to sit *outside* every specification table:
  `extensions(Regime::OwlRl)` returns it, `rules()` and `implemented()` are
  still exactly the same 78 and return none of it, `RuleId::is_extension`
  decides which is which, and every rendered report carries an
  `extension ext-eq-diff-sym` line beside its `missing` lines. So the closure a
  caller gets is Tables 4–9 plus a list it can read and reject, and
  `OWL-RL 78 / 78` remains a claim about Tables 4–9 and nothing else.

The live scoreboard is
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md).
