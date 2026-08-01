<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Entailment Rule Inventory

**This file is generated. Do not edit it by hand.** It is emitted by
`cargo run -p purrdf-entail --example gen_rule_inventory` from
`purrdf_entail::RuleId`, `rules(regime)` and `implemented(regime)`, and
`scripts/check-generated.sh` fails the build when the committed copy and a fresh
run disagree. Regenerate with `make metadata`.

**Defined** is the rule table the specification defines the regime by
(`rules(regime)`). **Implemented** is the subset this workspace's evaluator
actually fires (`implemented(regime)`). Their difference is the regime's gap,
and it is the same set a `ReasoningReport` names under `missing`.

Neither column counts an **extension** — a rule this workspace fires that no
specification table states. Those are listed in their own section below
(`extensions(regime)`), never folded into a coverage number, so a figure like
`OWL-RL 78 / 78` stays a claim about OWL 2 Profiles §4.3 Tables 4–9 and about
nothing else.

## `78 / 78` and `50 / 50` are two different measurements

This page is the RULE INVENTORY: `78 / 78` says every rule OWL 2 Profiles §4.3
Tables 4–9 states is one the chase fires. It says nothing about how many
published entailments that reaches, and the two figures are measured against
different things and can move independently.

The second measurement is ENTAILMENT CONFORMANCE, over the vendored W3C OWL 2 RL
entailment corpus: 50 of 50 cases agree with W3C's published verdict, 27 of 27
positive and 23 of 23 negative, with an empty divergence ledger. That figure is
`crates/sparql-conformance/entailment-suite/w3c-owl2-rl/`'s and is bounded by
what is vendored there — see `docs/CONFORMANCE.md`, which carries it beside the
corpus it was measured on.

Fifteen of those 50 are reached by a mechanism the rule table has no head for:
refutation, freeze-and-chase, comprehension, reflexivity and datatype
containment, each documented on `purrdf_entail::EntailmentMechanism`. NONE of
them adds a rule, which is why this inventory is byte-for-byte what it was
before they existed — they change how many times the table is run and what its
`false` is read as, not what the table states.

## Coverage by regime

| Regime | `--regime` | Defined | Implemented |
| --- | --- | ---: | ---: |
| Simple | `simple` | 0 | 0 |
| RDF | `rdf` | 3 | 3 |
| RDFS | `rdfs` | 18 | 18 |
| OWL-RL | `owl-rl` | 78 | 78 |
| OWL-Direct | `owl-direct` | 0 | 0 |
| RIF | `rif` | 0 | 0 |
| D | `d` | 5 | 5 |

A regime with a zero-length rule table is one this crate does not enumerate
rules for: `Simple` is the identity closure, and `OWL-Direct` and `RIF` are
served by a tableau and by a caller-supplied rule set respectively, neither of
which is a fixed table.

## Extensions

A rule this workspace's evaluator fires that **no specification table states**.
An extension appears in neither column above, for any regime: `rules(regime)` and
`implemented(regime)` name only specification rules, and `extensions(regime)`
names only these. `RuleId::is_extension` decides which is which, and a
`ReasoningReport` renders the list under `extension` beside the `missing` list —
so a caller that must act only on normative conclusions can tell from the report
rather than from prose.

Every entry is sound under the semantics of the vocabulary it reads; that is the
only standard a rule with no specification to appeal to can meet.

| Regime | `--regime` | Rule |
| --- | --- | --- |
| OWL-RL | `owl-rl` | `ext-eq-diff-sym` |

## RDF — 3 of 3 rules implemented

| Rule | Specification | Implemented |
| --- | --- | :---: |
| `rdfD1` | RDF 1.2 Semantics §8.1.1 (RDF patterns) | yes |
| `rdfD1a` | RDF 1.2 Semantics §8.1.1 (RDF patterns) | yes |
| `rdfD2` | RDF 1.2 Semantics §8.1.1 (RDF patterns) | yes |

## RDFS — 18 of 18 rules implemented

| Rule | Specification | Implemented |
| --- | --- | :---: |
| `rdfD1` | RDF 1.2 Semantics §8.1.1 (RDF patterns) | yes |
| `rdfD1a` | RDF 1.2 Semantics §8.1.1 (RDF patterns) | yes |
| `rdfD2` | RDF 1.2 Semantics §8.1.1 (RDF patterns) | yes |
| `rdfs1` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs2` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs3` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs4` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs5` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs6` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs7` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs8` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs9` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs10` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs11` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs12` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs13` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs14` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |
| `rdfs14a` | RDF 1.2 Semantics §9.2.1 (RDFS patterns) | yes |

## OWL-RL — 78 of 78 rules implemented

| Rule | Specification | Implemented |
| --- | --- | :---: |
| `eq-ref` | OWL 2 Profiles §4.3 Table 4 (Equality) | yes |
| `eq-sym` | OWL 2 Profiles §4.3 Table 4 (Equality) | yes |
| `eq-trans` | OWL 2 Profiles §4.3 Table 4 (Equality) | yes |
| `eq-rep-s` | OWL 2 Profiles §4.3 Table 4 (Equality) | yes |
| `eq-rep-p` | OWL 2 Profiles §4.3 Table 4 (Equality) | yes |
| `eq-rep-o` | OWL 2 Profiles §4.3 Table 4 (Equality) | yes |
| `eq-diff1` | OWL 2 Profiles §4.3 Table 4 (Equality) | yes |
| `eq-diff2` | OWL 2 Profiles §4.3 Table 4 (Equality) | yes |
| `eq-diff3` | OWL 2 Profiles §4.3 Table 4 (Equality) | yes |
| `prp-ap` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-dom` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-rng` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-fp` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-ifp` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-irp` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-symp` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-asyp` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-trp` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-spo1` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-spo2` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-eqp1` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-eqp2` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-pdw` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-adp` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-inv1` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-inv2` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-key` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-npa1` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `prp-npa2` | OWL 2 Profiles §4.3 Table 5 (Property Axioms) | yes |
| `cls-thing` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-nothing1` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-nothing2` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-int1` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-int2` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-uni` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-com` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-svf1` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-svf2` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-avf` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-hv1` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-hv2` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-maxc1` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-maxc2` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-maxqc1` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-maxqc2` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-maxqc3` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-maxqc4` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cls-oo` | OWL 2 Profiles §4.3 Table 6 (Classes) | yes |
| `cax-sco` | OWL 2 Profiles §4.3 Table 7 (Class Axioms) | yes |
| `cax-eqc1` | OWL 2 Profiles §4.3 Table 7 (Class Axioms) | yes |
| `cax-eqc2` | OWL 2 Profiles §4.3 Table 7 (Class Axioms) | yes |
| `cax-dw` | OWL 2 Profiles §4.3 Table 7 (Class Axioms) | yes |
| `cax-adc` | OWL 2 Profiles §4.3 Table 7 (Class Axioms) | yes |
| `dt-type1` | OWL 2 Profiles §4.3 Table 8 (Datatypes) | yes |
| `dt-type2` | OWL 2 Profiles §4.3 Table 8 (Datatypes) | yes |
| `dt-eq` | OWL 2 Profiles §4.3 Table 8 (Datatypes) | yes |
| `dt-diff` | OWL 2 Profiles §4.3 Table 8 (Datatypes) | yes |
| `dt-not-type` | OWL 2 Profiles §4.3 Table 8 (Datatypes) | yes |
| `scm-cls` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-sco` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-eqc1` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-eqc2` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-op` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-dp` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-spo` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-eqp1` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-eqp2` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-dom1` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-dom2` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-rng1` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-rng2` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-hv` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-svf1` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-svf2` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-avf1` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-avf2` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-int` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |
| `scm-uni` | OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary) | yes |

## D — 5 of 5 rules implemented

| Rule | Specification | Implemented |
| --- | --- | :---: |
| `dt-type1` | OWL 2 Profiles §4.3 Table 8 (Datatypes) | yes |
| `dt-type2` | OWL 2 Profiles §4.3 Table 8 (Datatypes) | yes |
| `dt-eq` | OWL 2 Profiles §4.3 Table 8 (Datatypes) | yes |
| `dt-diff` | OWL 2 Profiles §4.3 Table 8 (Datatypes) | yes |
| `dt-not-type` | OWL 2 Profiles §4.3 Table 8 (Datatypes) | yes |
