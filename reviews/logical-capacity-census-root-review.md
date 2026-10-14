# Independent review of the initial census implementation

Verdict: REVISE. The five-file integration uses the existing parsed shipping source walk, RuleHit/index/check wiring and enforced ledger job. The initial five seeded tests and strict compilation pass, but do not resolve the following coverage defects found in the actual implementation.

1. `Capacity::proof` substitutes u64::MAX for an unresolved slot limit. This can incorrectly certify a NonZeroU64 construction when a dynamic u128 limit exceeds its representation. An unknown bound must supply no proof. Add rejected unknown and above-width controls; only recognize bounded expressions with actual structural evidence.
2. Compound assignment invalidation covers addition/subtraction/multiplication/division but omits remainder, bitwise and shift assignments. All can invalidate the checked index. Add a rejected bitwise mutation control and handle every compound assignment.
3. Imported associated-operation aliases lose their qualified parent. `use purrdf_lex::allocation::CapacityError::slot as reserve` may evade the typed sink rule because only `slot` survives. Preserve qualified operation provenance and seed direct and nested/grouped import variants.
4. The unchanged helper gate reports two duplicate visitor-body groups introduced by this rule. Consolidate through the existing visitor homes; do not weaken fingerprint enforcement to accommodate copied bodies.

The first complete-tree gate reports46 candidates. These are not46 established production defects. Each needs source disposition distinguishing an actual data-dependent logical refusal from a checked representation, bounded codec conversion, physical allocation failure or fixture reachability. Current-tree zero is not acceptance if obtained by broad function exemptions, name-only proofs or tuning away unresolved cases.

All four findings were delivered to the implementer and acknowledged. Requalification of corrected seeds, strict compilation, actual complete-tree findings and unchanged gate is required before acceptance. No production core/Datalog defect or whole-pair PASS is claimed by this review. The existing full gate terminal0 qualifies published production7a3f96ef7, not this uncommitted new checker.
