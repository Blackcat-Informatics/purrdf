// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A downstream can NAME every type the reasoning surface hands it, from `purrdf`
//! alone.
//!
//! The umbrella's standing claim is that "anything a consumer legitimately imports
//! is reachable from `purrdf` alone — never by reaching into a sub-crate". Reaching
//! a *value* is not enough: matching on an error variant, storing a budget, or
//! comparing a contract hash all require WRITING the type's name, and a name that
//! only exists in an undeclared crate cannot be written.
//!
//! `purrdf-entail`'s public surface carries three `purrdf-datalog` types across the
//! umbrella boundary — `seminaive::EvalError` (on
//! [`purrdf::entail::EntailError::Evaluate`]), `cache::ContractHash` and
//! `seminaive::BudgetReport` (both handed out by
//! [`purrdf::entail::ReasoningReport`]). Every one of them is spelled below through
//! `purrdf::` and nothing else, in a *type-annotated* binding rather than an
//! inferred one, so deleting the `purrdf::datalog` module fails this file to
//! compile instead of quietly leaving the claim false.
//!
//! This is a `tests/` integration target on purpose: it links against `purrdf` the
//! way a downstream does, with only the umbrella in its `extern` set.

use purrdf::datalog::cache::ContractHash;
use purrdf::datalog::seminaive::{BudgetReport, BudgetResource, EvalError};
use purrdf::entail::RuleSet;
use purrdf::entail::{
    Completeness, Construct, EntailError, Materialization, ReasoningReport, RuleId, materialize,
};
use purrdf::{RdfDataset, ReasoningError};

use std::sync::Arc;

/// One individual in two disjoint classes — enough for `cax-dw` to refuse.
const INCONSISTENT: &str = concat!(
    "<http://example.org/A> <http://www.w3.org/2002/07/owl#disjointWith> ",
    "<http://example.org/B> .\n",
    "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ",
    "<http://example.org/A> .\n",
    "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ",
    "<http://example.org/B> .\n",
);

/// `ex:A ⊑ ex:B` with one typed instance — enough for `rdfs9` to fire.
const SCHEMA: &str = concat!(
    "<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> ",
    "<http://example.org/B> .\n",
    "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ",
    "<http://example.org/A> .\n",
);

/// Every type the public reasoning surface carries is nameable through `purrdf`.
///
/// The annotations are the assertion. `let hash = report.contract_hash();` would
/// compile with no `purrdf::datalog` module at all (inference needs no path); the
/// explicit `: ContractHash` is what makes the re-export load-bearing.
#[test]
fn the_umbrella_names_every_type_the_reasoning_surface_carries() {
    let dataset: Arc<RdfDataset> =
        purrdf::parse_dataset(SCHEMA.as_bytes(), "application/n-quads", None)
            .expect("N-Quads through the umbrella");

    let (closure, report): (Arc<RdfDataset>, ReasoningReport) =
        materialize(&dataset, Materialization::Rdfs).expect("the RDFS closure");
    assert!(closure.quad_count() >= dataset.quad_count());

    // ── the three `purrdf-datalog` types the report carries ────────────────
    let hash: ContractHash = report.contract_hash();
    assert_eq!(hash.to_hex().len(), 64);
    let budget: BudgetReport = report.budget();
    assert!(budget.join_steps() > 0);

    // ── the `purrdf-entail` types alongside them ───────────────────────────
    let completeness: &Completeness = report.completeness();
    let missing: &[RuleId] = completeness.missing();
    let fired: &[(RuleId, u64)] = report.rules_fired();
    assert!(!fired.is_empty(), "rdfs9 fires on this schema");
    // The gap is data, whatever its current size; naming `RuleId` is the point.
    let _ = missing.len();
    for boundary in report.boundaries() {
        let construct: Construct = boundary.construct();
        assert!(!construct.as_str().is_empty());
    }

    // ── the error side, including its datalog payload ──────────────────────
    //
    // A disjointness violation, which is what an `EntailError` from a well-formed call
    // now looks like: no regime is refused, so the error side of `materialize` is
    // reached with BAD DATA rather than with an unserved regime.
    let clashing: Arc<RdfDataset> =
        purrdf::parse_dataset(INCONSISTENT.as_bytes(), "application/n-quads", None)
            .expect("N-Quads through the umbrella");
    let refused: EntailError =
        materialize(&clashing, Materialization::OwlRl).expect_err("a disjointness clash");
    // A consumer matching on the carried `EvalError` must be able to write BOTH
    // names; the budget-exhaustion arm is the one that carries it.
    let ceiling: Option<(BudgetResource, BudgetReport)> =
        if let EntailError::Evaluate(EvalError::BudgetExhausted { resource, report }) = &refused {
            Some((*resource, *report))
        } else {
            None
        };
    assert!(
        ceiling.is_none(),
        "the clash is Inconsistent, not a ceiling refusal"
    );
    assert!(matches!(refused, EntailError::Inconsistent(_)));

    // The umbrella's own reasoning façade wraps the same error type.
    let wrapped: ReasoningError = ReasoningError::from(refused);
    assert!(matches!(wrapped, ReasoningError::Entailment(_)));

    // …and the two lanes that used to be refused here now ANSWER through the same
    // umbrella-visible entry point, each carrying its own input.
    for plan in [
        Materialization::OwlDirect(&[]),
        Materialization::Rif(&RuleSet::new()),
    ] {
        let (_, report) = materialize(&dataset, plan).expect("a served regime");
        assert_eq!(report.regime(), plan.regime());
    }
}

/// The umbrella carries the datalog engine's own entry points too, not just the
/// three types the entailment surface leaks — a consumer that reads a contract
/// hash may legitimately want to mint one.
#[test]
fn the_umbrella_reaches_the_datalog_engine_itself() {
    // Module paths, spelled through `purrdf::datalog` alone.
    let store: purrdf::datalog::store::RelationStore = purrdf::datalog::store::RelationStore::new();
    assert!(!format!("{store:?}").is_empty());
    let mint: fn(&[purrdf::datalog::clause::DlClause]) -> ContractHash =
        purrdf::datalog::cache::contract_hash;
    // An empty program still has an identity, and it is a `ContractHash`.
    let identity: ContractHash = mint(&[]);
    assert_eq!(identity.to_hex().len(), 64);
}
