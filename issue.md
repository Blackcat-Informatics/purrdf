# Issue #514: datalog: never-derive declarations -- refuse a rule set that can reach an assessor-only predicate

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

**Never-derive declarations** for rule sets. A set of predicates that may only be *asserted*, by an assessor, and never *derived* by a rule. Registering or evaluating a rule set must check reachability over its rule heads, transitively, and refuse a rule set that could derive a declared predicate, naming the path.

## Why

GMEOW already has one such obligation: `logic:IntentNotFromDeceptionObligation`. No rule anywhere may derive `gmeow:deceptiveIntentClaim`; intent is an assessor's defeasible attribution, never an entailment (`gmeow-ontology` `slices/core/deception/docs.md`, and conformance case `conformance/logic/cases/deception/held-projected-no-intent/`). gmeow enforces it in its own `make verify`.

Guilt, diagnosis and motive have the same shape. Katamari's rule sets (ADR-0045 45.4a, Proposed) need the check at registration. Patrick has ruled that declarations come from more than one place: the ontology, the store's profile and the rule set itself. The set in force is the **union**, and no layer can withdraw another's.

## What it needs

- A declaration input to rule-set admission: a set of predicate IRIs, possibly assembled from several sources, with the source of each recorded.
- Static reachability over the dependency graph of rule heads, including through intermediate derived predicates and through the chase's existential heads. A rule set that can reach a declared predicate is refused, and the refusal names the chain of rules.
- The check is total and exact. A rule set the analysis cannot decide is refused, never admitted.

## Acceptance criteria

- GMEOW's deception case is reproduced: a rule set deriving `gmeow:deceptiveIntentClaim` directly, or through an intermediate predicate, is refused with the path.
- An unrelated rule set over the same data is admitted.
- The result is deterministic.


## Comments (0)

