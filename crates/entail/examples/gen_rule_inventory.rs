// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Regenerates the committed entailment rule inventory
//! (`docs/book/src/entailment-rules.md`) from the crate's own public API.
//!
//! The table is READ from [`purrdf_entail::RuleId`], [`purrdf_entail::rules`],
//! [`purrdf_entail::implemented`] and [`purrdf_entail::extensions`] rather than
//! transcribed, so a rule added to or removed from the calculus changes the document
//! mechanically. `scripts/check-generated.sh`
//! re-runs this and diffs the result against the committed file, which is what stops the
//! prose and the code from drifting apart again: a coverage claim in this repository is
//! now a build artifact, not an assertion someone has to remember to update.
//!
//! Run via `make metadata` (writes) or `make check` (verifies). Output goes to stdout.

use std::fmt::Write as _;

use purrdf_entail::{Regime, RuleId, extensions, implemented, rules};

/// Every [`Regime`], in the order the document presents them.
///
/// The exhaustive match in [`regime_name`] is what forces this list to be revisited when
/// `Regime` grows a variant: a new variant fails to compile there, not here.
const ALL_REGIMES: [Regime; 7] = [
    Regime::Simple,
    Regime::Rdf,
    Regime::Rdfs,
    Regime::OwlRl,
    Regime::OwlDirect,
    Regime::Rif,
    Regime::D,
];

/// The regime's display name. Exhaustive over [`Regime`] on purpose.
fn regime_name(regime: Regime) -> &'static str {
    match regime {
        Regime::Simple => "Simple",
        Regime::Rdf => "RDF",
        Regime::Rdfs => "RDFS",
        Regime::OwlRl => "OWL-RL",
        Regime::OwlDirect => "OWL-Direct",
        Regime::Rif => "RIF",
        Regime::D => "D",
    }
}

/// The `--regime` / `Regime.<NAME>` spelling every host accepts for `regime`.
fn regime_token(regime: Regime) -> &'static str {
    match regime {
        Regime::Simple => "simple",
        Regime::Rdf => "rdf",
        Regime::Rdfs => "rdfs",
        Regime::OwlRl => "owl-rl",
        Regime::OwlDirect => "owl-direct",
        Regime::Rif => "rif",
        Regime::D => "d",
    }
}

/// The specification section each rule-name prefix belongs to.
///
/// This is the one mapping the generator supplies rather than reads, because the crate
/// exposes rule NAMES and not the table each name was drawn from. It is keyed on the
/// prefix the specifications themselves use to name their tables — OWL 2 Profiles §4.3
/// splits Tables 4–9 by exactly these six prefixes, and RDF 1.2 Semantics names its two
/// pattern sets `rdfD*` and `rdfs*` — so it is a total function on the rule set rather
/// than a per-rule list that could fall out of date. An unrecognized prefix is a hard
/// error (see [`citation`]), not a blank cell.
const CITATIONS: [(&str, &str); 8] = [
    ("eq-", "OWL 2 Profiles §4.3 Table 4 (Equality)"),
    ("prp-", "OWL 2 Profiles §4.3 Table 5 (Property Axioms)"),
    ("cls-", "OWL 2 Profiles §4.3 Table 6 (Classes)"),
    ("cax-", "OWL 2 Profiles §4.3 Table 7 (Class Axioms)"),
    ("dt-", "OWL 2 Profiles §4.3 Table 8 (Datatypes)"),
    ("scm-", "OWL 2 Profiles §4.3 Table 9 (Schema Vocabulary)"),
    ("rdfD", "RDF 1.2 Semantics §8.1.1 (RDF patterns)"),
    ("rdfs", "RDF 1.2 Semantics §9.2.1 (RDFS patterns)"),
];

/// The specification citation for `rule`.
///
/// Refuses rather than guesses: a rule id whose prefix is not in [`CITATIONS`] means the
/// calculus grew a table this generator does not know how to cite, and emitting a row
/// with an empty citation would put exactly the kind of unsourced claim into the document
/// that this artifact exists to prevent.
fn citation(rule: RuleId) -> Result<&'static str, String> {
    let name = rule.as_str();
    CITATIONS
        .iter()
        .find(|(prefix, _)| name.starts_with(prefix))
        .map(|(_, section)| *section)
        .ok_or_else(|| format!("no specification citation is registered for rule `{name}`"))
}

/// Render the whole document.
fn render() -> Result<String, String> {
    let mut out = String::new();
    out.push_str(
        "<!--\n\
         SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>\n\
         SPDX-License-Identifier: CC-BY-4.0\n\
         -->\n\n\
         # Entailment Rule Inventory\n\n\
         **This file is generated. Do not edit it by hand.** It is emitted by\n\
         `cargo run -p purrdf-entail --example gen_rule_inventory` from\n\
         `purrdf_entail::RuleId`, `rules(regime)` and `implemented(regime)`, and\n\
         `scripts/check-generated.sh` fails the build when the committed copy and a fresh\n\
         run disagree. Regenerate with `make metadata`.\n\n\
         **Defined** is the rule table the specification defines the regime by\n\
         (`rules(regime)`). **Implemented** is the subset this workspace's evaluator\n\
         actually fires (`implemented(regime)`). Their difference is the regime's gap,\n\
         and it is the same set a `ReasoningReport` names under `missing`.\n\n\
         Neither column counts an **extension** — a rule this workspace fires that no\n\
         specification table states. Those are listed in their own section below\n\
         (`extensions(regime)`), never folded into a coverage number, so a figure like\n\
         `OWL-RL 78 / 78` stays a claim about OWL 2 Profiles §4.3 Tables 4–9 and about\n\
         nothing else.\n\n\
         ## `78 / 78` and `50 / 50` are two different measurements\n\n\
         This page is the RULE INVENTORY: `78 / 78` says every rule OWL 2 Profiles §4.3\n\
         Tables 4–9 states is one the chase fires. It says nothing about how many\n\
         published entailments that reaches, and the two figures are measured against\n\
         different things and can move independently.\n\n\
         The second measurement is ENTAILMENT CONFORMANCE, over the vendored W3C OWL 2 RL\n\
         entailment corpus: 50 of 50 cases agree with W3C's published verdict, 27 of 27\n\
         positive and 23 of 23 negative, with an empty divergence ledger. That figure is\n\
         `crates/sparql-conformance/entailment-suite/w3c-owl2-rl/`'s and is bounded by\n\
         what is vendored there — see `docs/CONFORMANCE.md`, which carries it beside the\n\
         corpus it was measured on.\n\n\
         Fifteen of those 50 are reached by a mechanism the rule table has no head for:\n\
         refutation, freeze-and-chase, comprehension, reflexivity and datatype\n\
         containment, each documented on `purrdf_entail::EntailmentMechanism`. NONE of\n\
         them adds a rule, which is why this inventory is byte-for-byte what it was\n\
         before they existed — they change how many times the table is run and what its\n\
         `false` is read as, not what the table states.\n\n",
    );

    out.push_str("## Coverage by regime\n\n");
    out.push_str("| Regime | `--regime` | Defined | Implemented |\n");
    out.push_str("| --- | --- | ---: | ---: |\n");
    for regime in ALL_REGIMES {
        let _ = writeln!(
            out,
            "| {} | `{}` | {} | {} |",
            regime_name(regime),
            regime_token(regime),
            rules(regime).len(),
            implemented(regime).len(),
        );
    }
    out.push_str(
        "\nA regime with a zero-length rule table is one this crate does not enumerate\n\
         rules for: `Simple` is the identity closure, and `OWL-Direct` and `RIF` are\n\
         served by a tableau and by a caller-supplied rule set respectively, neither of\n\
         which is a fixed table.\n",
    );

    out.push_str("\n## Extensions\n\n");
    out.push_str(
        "A rule this workspace's evaluator fires that **no specification table states**.\n\
         An extension appears in neither column above, for any regime: `rules(regime)` and\n\
         `implemented(regime)` name only specification rules, and `extensions(regime)`\n\
         names only these. `RuleId::is_extension` decides which is which, and a\n\
         `ReasoningReport` renders the list under `extension` beside the `missing` list —\n\
         so a caller that must act only on normative conclusions can tell from the report\n\
         rather than from prose.\n\n\
         Every entry is sound under the semantics of the vocabulary it reads; that is the\n\
         only standard a rule with no specification to appeal to can meet.\n\n",
    );
    let extended: Vec<(Regime, &[RuleId])> = ALL_REGIMES
        .into_iter()
        .map(|regime| (regime, extensions(regime)))
        .filter(|(_, list)| !list.is_empty())
        .collect();
    if extended.is_empty() {
        out.push_str("No regime declares one.\n");
    } else {
        out.push_str("| Regime | `--regime` | Rule |\n");
        out.push_str("| --- | --- | --- |\n");
        for (regime, list) in extended {
            for rule in list {
                let _ = writeln!(
                    out,
                    "| {} | `{}` | `{}` |",
                    regime_name(regime),
                    regime_token(regime),
                    rule.as_str(),
                );
            }
        }
    }

    for regime in ALL_REGIMES {
        let defined = rules(regime);
        if defined.is_empty() {
            continue;
        }
        let fired = implemented(regime);
        let _ = write!(
            out,
            "\n## {} — {} of {} rules implemented\n\n",
            regime_name(regime),
            fired.len(),
            defined.len(),
        );
        out.push_str("| Rule | Specification | Implemented |\n");
        out.push_str("| --- | --- | :---: |\n");
        for &rule in defined {
            let _ = writeln!(
                out,
                "| `{}` | {} | {} |",
                rule.as_str(),
                citation(rule)?,
                if fired.contains(&rule) { "yes" } else { "no" },
            );
        }
    }

    Ok(out)
}

fn main() {
    match render() {
        Ok(document) => print!("{document}"),
        Err(message) => {
            eprintln!("gen_rule_inventory: {message}");
            std::process::exit(2);
        }
    }
}
