# Issue #452: owl:propertyChainAxiom is a property-chain boundary; regular chains are not decided

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

Any `owl:propertyChainAxiom` makes `Reasoner::consistency` report `completeness decided-within-boundaries` with a `property-chain` boundary. Chains are not part of the decided fragment in 3.0.0.

## Reproduction (purrdf 3.0.0 from crates.io)

```
<http://example.org/p> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#ObjectProperty> .
<http://example.org/q> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#ObjectProperty> .
<http://example.org/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#ObjectProperty> .
<http://example.org/r> <http://www.w3.org/2002/07/owl#propertyChainAxiom> _:l1 .
_:l1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <http://example.org/p> .
_:l1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> _:l2 .
_:l2 <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <http://example.org/q> .
_:l2 <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> .
```

Result: `true`, `decided-within-boundaries`, boundaries `{property-chain}`.

## Expected

OWL 2 DL (SROIQ) admits regular role-inclusion chains (`p ∘ q ⊑ r` under the regularity condition). These should be decided exactly. A chain that violates regularity should be refused, as non-OWL 2 DL input already is.

## Impact

The gmeow-ontology production class world `graph/logic` states 9 property chains. Each one keeps that world's consistency verdict from being exact.


## Comments (0)

