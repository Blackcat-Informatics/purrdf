# Scoped glossary qualification corrections

The initial full gate found two owned hygiene defects: broad ASCII-whitespace
recognition in Markdown projection and a duplicated test fixture writer. The
correction uses the shared exact terminal class at the specific CommonMark link
and fence boundaries, with clause references and form-feed neighbor regressions.
Tests use the existing filesystem operations directly instead of a second helper.

Eleven glossary unit tests, both actual caller/parity tests, strict all-target
helper clippy and the complete live shared-helper census passed. Independent
correction review PASS; normal commit hooks passed for b1833aeed.

Initial full exit2 and focused private-cache admission exit101 remain preserved.
Cargo now creates its own private build/target cache markers; no testkit fallback,
synthetic marker, gate exemption or hook bypass was introduced. A crossed
coordination hold aborted one corrected full attempt with exit143, which is
neither a source failure nor a pass. Settled full and complete rendering gates
remain pending; this correction is not whole-issue completion.
