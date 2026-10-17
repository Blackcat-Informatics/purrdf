# Reuse the complete current-head measurements

Workflow 39006247060 completed at 2c9c08dc50099c01c17f847156512fef94be6f6c.
ci-final-run.json and ci-final-jobs.json bind the seven assembly configurations
to that source. Three configuration jobs pass. Four reach document comparison
and report only five stale cells, preserved in the raw current-head job logs.
The gate evaluates kernel constraints and source/compiler stability before
document comparison; no missing symbol or kernel constraint fails in those jobs.

The existing check-simd-asm.py write_doc implementation regenerated those five
cells, using their exact measured values from the same-head job logs. Other
cells are unchanged and agree with the complete CI matrix's parity results.
The existing doc_checks implementation passes complete coverage/parity on the
result. ci-projection-cells.json records every old/new value and configuration.
The source diff changes only two table rows plus the requested provenance text.
No tool, manifest, threshold, compiler/cache setting or report status was edited.
Failed/incomplete reports are not retagged or used as passed merge reports.

The duplicate local write-doc run was interrupted through its verified owned
launcher after x86 and x86-v3 and during x86-v4; session 65017 exits 130. Its
log is retained and is NOT a passed full matrix. Its Runner performs its normal
child cleanup. This avoids repeating all seven already completed CI compilations.
Furthermore, source_identity fingerprints documentation too: the provenance
wrapping edit made during that run would invalidate its final source check.
The earlier assumption that frozen Rust alone preserved its identity was wrong.
Do not count that run as qualification or repeat it to obtain the same counts.

Normal commit hooks remain required. After publication, final CI must verify
the regenerated document on the actual candidate before ghprsq integration.
The real full-base review remains applicable to unchanged Rust and its requested
provenance correction; any new feedback still requires disposition.
