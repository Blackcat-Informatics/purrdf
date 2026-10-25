# Issue #710: Move RDFC n-degree search onto heap frames

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Parent: #489.

RDFC's n-degree search recursively calls itself for newly issued related
blank nodes. A caller-removable work budget must not leave native recursion
as an accidental implementation ceiling.

Replace that recursion with explicit owned heap frames on the production path.
Preserve the exact RDFC traversal, permutation/pruning order, issuer ownership,
hash choice, successful bytes, existing call/permutation charging and stop
checks. Do not retain a second recursive implementation or add a dependency.

Acceptance: existing native canonicalization/profile and W3C conformance cases
retain exact output and refusal behavior; a focused deep n-degree case exercises
the iterative path; affected strict compilation, helper proofs, normal hooks,
independent review and hosted gates pass. No general-semantic WASM copies.

This unit does not introduce configurable budgets, change profile identity,
port dataset-state controls or implement heap UDF execution. Those requirements
remain in #489, which stays open. Budget this coherent delivery at no more than
four active engineering/local-validation hours, reassessing at two hours.


## Comments (0)

