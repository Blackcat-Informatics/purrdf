# CodeQL synthetic assertion dispositions

The current-head CodeQL alert check originally failed on rust/cleartext-logging
alerts241/242. Both instances are classified by the analyzer as test code.

241: crates/entail/tests/reasoner.rs1690-1691 prints
ProfileCertificate::violations() only when a fixed example.org regular-chain
classification assertion fails. The certificate represents OWL profile membership;
the payload is typed ProfileViolation data. It is not a cryptographic certificate,
credential, private key or runtime user input.

242: crates/entail/tests/regular_role_chains.rs100-101 prints
DlCertificate::boundaries() only when completeness of a synthetic ontology
reasoning answer differs from Decided. Those boundaries classify unsupported
reasoning constructs. The fixture builds its own example.org ontology; no secret
or authentication certificate enters this diagnostic.

Both alerts were dismissed as false positives with explicit reasons through the
GitHub code-scanning API. The API confirmed state=dismissed for both, and the
subsequent complete PR checks surface showed CodeQL passing. The Rust CodeQL
analysis itself also passed. No code suppression, source deletion, renamed payload
or disabled security check was used. Useful assertion diagnostics remain intact.

The complete seven-configuration assembly matrix and aggregate simd-asm also pass
on the current head. Only library and integration-5 runtime jobs were live at this
checkpoint; no final merge readiness or issue closure is claimed here.
