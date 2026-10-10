# Actual #508/#499 native combination

Original `main-508-integration-family-3.sh` executed under
`/opt/purrdf-schema-499-qualification/run` (4 compiler/test jobs,
16GiB MemoryMax, zero swap), original session24547, terminal exit0.
Actual log: `main-508-integration-family-3.log`.

- Managed strict all-target Clippy for XSD, entailment, validation, SPARQL
  evaluator and C ABI: PASS (1m42s), retaining `--locked -- -D warnings`.
- Every affected XSD/entail/validate native library and integration target:
  PASS, including710 entailment unit cases, the full independent support/proof
  tests, public cardinality boundary and original byte goldens.
- The new public unequal-scale513-digit decimal range control: PASS,
  exact empty interval and actual allocation-count/peak/retained/release
  assertions; all prior range refusal and cleanup-error precedence controls PASS.
- All59 selected SPARQL numeric production cases: PASS, including aggregate
  physical ownership, comparisons, casts, contracts, governors and parallel/
  frozen numeric transcript controls.
- All104 C ABI library cases: PASS, including unchanged16-case proof/check
  golden byte identity.
- Complete shared-helper hygiene and registered hash-domain gate: PASS,
  no new exemption;86 domains and3 original exceptions current.
- Fresh source-built Python3.13.12 native binding: build PASS; all70 reasoning
  consumer cases PASS (0.39s), including original proof/check golden bytes.

The full affected family is terminal PASS. Historical attempts1 and2 remain
failed records with their actual corrections. The original full4 gate is not
relabelled as execution of this combined tree. Actual affected portable numeric
and fresh optimized package proof/checker execution has also terminated PASS,
exit0, original session50582; `main-508-portable-family-result.md` binds its5
numeric cases, fresh optimized package and all43 public entailment cases.
Independent source resolution review has no source finding
(`reviews/main-508-resolution-review.md`); the parent read both terminal logs
and issued PASS for the actual affected integration. Normal signed
synchronization hooks/push and fresh hosted/candidate acceptance
remain pending. No issue closure is claimed.
