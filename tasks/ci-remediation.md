# Hosted findings and bounded repair

Five hosted jobs failed on d0f544f49. Complete job logs are ci-cross-arch.log,
ci-lib.log and ci-integration-{2,3,4}.log. Source review itself found no actionable
correctness/availability issue. Required CI is not green and merge is blocked.

1. Cross-arch and library tests identify the same production mismatch:
   base_cardinality saturates merged-scope counts, but cost_base still uses an
   overflowing addition. Make cost_base saturate identically; retain the existing
   boundary test, including u64::MAX inputs. Do not restore a panic merely to
   reproduce old behavior.
2. The evaluator trace reports nine intentional candidate-fuel changes in the
   RDF 1.2 corpus under profile 19, with unchanged answers/intermediate counts.
   Execute the existing query-trace generator, inspect the complete diff and
   recheck original corpus/trace inputs. Do not edit inputs, acceptance or
   expected answers to hide a regression.
3. Operational/segmented EXPLAIN tests compare whole receipts to native bytes.
   The new access descriptor is intentionally backend-specific. Centralize its
   formatting in StatementIndexSupport's Display, assert scan-default explicitly
   on those adapters and compare every other receipt byte unchanged. Existing
   admission, allocation, clone/last-owner and first-fault assertions remain.

The manual full run was still compiling this now-known failing head. Its unique
process group 1745961 was inspected and terminated for source repair, exit 143;
the geographic process, sibling work and hooks were untouched. This is not a
passing full gate. No executing launcher was edited or restarted to observe it.

Targeted repair checks run first. The required final full qualification remains
mandatory after these fixes settle; a repeated mandatory execution is required
because this first execution did not qualify the source, not because a Stage
phase advanced. Reuse unchanged large-population/instruction/kernel evidence:
the fix affects overflowing synthetic merged counts and receipt presentation,
not the measured single-graph range/query computations or populations.

The bounded production saturation repair passes the original merged-count
boundary test, including u64::MAX. All twelve directly affected EXPLAIN cases
pass with every receipt byte other than the explicitly asserted access descriptor
preserved. Strict core/evaluator library and test Clippy passes with warnings
denied. The shared-helper gate passes after refreshing only the forty existing
constructor closure identities for the descriptor-formatting move. All proof
contracts and authority identities are unchanged, with 158 constructor source
files. No checker or acceptance changed.

The existing trace generator completed successfully. Review confirms exactly
nine changed cases among 712; all complete runs use less fuel, all solution
bytes and non-candidate node charges are unchanged. Changed budget-prefix
progression follows the removed candidate charges. The original query and update
trace checks both pass, with their two regeneration paths ignored as intended.
See repair-trace-diff-review.txt and repair-trace-{regenerate,check}.log.
No corpus input, answer golden, governor price or acceptance threshold changed.
