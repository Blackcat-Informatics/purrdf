# Hosted review gap assessment

Current hosted review covers the complete published PR source through 279a26a0e.
The raw inline comments and full review body are retained under tasks/review-*.json.
This assessment verifies those observations against actual source; it is owner
judgment, not a second independent review. No subagents are used.

1. Paged statistics getter: VALID. failed() only reads the sticky latch, whereas
   read_error() checks the current provider snapshot. The new getter could return
   metadata after unobserved drift. Change it to the existing readiness path and
   prove that restoring the provider cannot clear the latched first cause.
2. Replay/global-domain alignment: NOT A DEFECT. The original main implementation
   already calibrated replay with each relation's min(base, global terms) domain.
   Its documentation explicitly distinguishes admission replay from order search's
   cheaper global-domain heuristic. Replacing the fallback with global terms
   discards this existing admission behavior and conflicts with retaining unknown-
   metadata fallback. Preserve it and pin the actual metadata-free 20/5/1-spoke
   example with seven drivers: final 7, peak 140. Post the computation/evidence
   in reply before resolving the review thread.
3. Unused statistics strings in admission: VALID, material despite its nit label.
   Ordinary intermediate-cell admission calls the same survey as EXPLAIN and
   currently recompiles/formats/charges evidence that only EXPLAIN reads. Select
   recording only from the actual EXPLAIN entry; preserve the shared forecast
   and prove equal peaks with evidence absent/present on the two survey routes.

The first and third fixes are in progress with meaningful regressions. Other
issue demonstrations remain valid for unchanged numerical and storage paths.
Final hosted checks and material finding dispositions remain open.
Replay fallback concern declined with computation and original-law rationale in discussion_r4288569397; thread resolved. The new explicit assertion remains to publish with the current remediation.
