Fix imported closure proof replay source identity

Closes #591
Defect-Class: source-identity

The CLI exported a resolved import closure as canonical N-Quads and reparsed it
for proof checking. Canonical ordering changed first-seen native term IDs: the
recorded individual no longer denoted the same source term. Keep the consumer's
resolved frozen dataset and build a fresh checking context through the shared
verifier. No producer search state or context participates in checking.

Files: validate/regime adds the snapshot checking entry and extracts one shared
body; CLI/consistency uses it; the existing public proof-vector target adds
independent import loads and negative neighbours; Makefile and WASM_TESTING
register that one additional portable case. Canonical export remains a data
export with unchanged bytes and corrected context advice. No proof kernel,
support comparison, encoding, golden, dependency or profile is changed.

Focused native/CLI/scalar-WASM, strict affected Clippy, helper hygiene and normal
commit/push hooks pass. The unchanged original CLI assertion passes. Existing
sixteen service vectors and five current-support controls remain passing.
No full local gate, external corpus, resource probe or performance timing was run.

Hosted checks and actual review remain PENDING. Owner assessments are identified
as such; no independent reviewer or approval is invented. No issue requirement
is deferred to follow-up. No code scope cut or deficiency entry applies.

Authoritative plan: .stage/proof-replay-imported-inconsistent/plan.md.
Selected evidence: .stage/proof-replay-imported-inconsistent.
Integration assessment currently has unchanged base and clean predicted tree;
refresh before final notes and ghprsq. This is a draft, not final merge readiness.
