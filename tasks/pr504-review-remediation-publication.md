Four review findings are confirmed against the production implementation. PR #504 remains open while they are repaired:

1. Select RDFLib graphs independently of physical storage-table order: remove excluded original values before inserting projections. Regressions will check selected triples, exclusion of other triples, and exact query-result multiplicity when ordinary/reifier/annotation rows overlap.
2. Refuse contextual Apply in the on-demand single-call read path while preserving its column/provenance summaries. Extend differential coverage to Apply policies.
3. Give mandatory Apply's RHS the mapped inputs whose drivers are certainly bound by its LHS or enclosing context. Preserve optional-left-only certainty; check a real bound-mode planning consumer and an unbound control.
4. Treat optional Apply RHS SERVICE occurrences as indirect in endpoint summaries. Check parent lookup behavior and keep mandatory Apply occurrences direct.

Validation will use focused native Rust regressions and independent review of the changed callers. Existing passing CI and analytical cost evidence will be reused only where the changes leave their claims applicable; changed paths require new evidence. No merge will occur with unresolved feedback or missing required checks. These fixes are part of the existing work, with no scope deferral.
