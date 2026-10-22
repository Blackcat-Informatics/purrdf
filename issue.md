# Issue #679: Entailment: bind frozen implication warrants to their original premise

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Frozen implication warrants currently verify against an empty or weakened replacement premise. The verifier only checks containment in the carried derived closure; an empty supplied premise satisfies that check vacuously.

Bind each frozen warrant to the original asserted default-graph premise and reject verification with a different premise. Reuse the already collected premise rather than cloning another copy or introducing another verifier.

Acceptance:

- A public property-chain implication produces a frozen warrant that verifies against its original premise.
- The same warrant rejects an empty premise and a strict weakened subset.
- Run the affected native and actual WebAssembly tests, normal repository hooks, and required CI/review.

Discovered while implementing #493; related to #501's snapshot-bound proof requirements. This independently useful correctness repair does not complete either parent's maintenance, proof-session, resource, or lifetime requirements. Both parents remain open with their full contracts.


## Comments (0)

