# Issue #1144: v4 API: bind proof checking to source and evaluated scope

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Programme: https://github.com/Blackcat-Informatics/purrdf/issues/1140

## Scope

Replace check_fol_proof's freely supplied Boolean negation oracle with typed program/source-bound checking evidence. Settle FolProof construction, inspection, ownership, conclusion access and identity/version contracts before v4. The checker must validate negative completion evidence rather than accept an unchecked evaluator Boolean. Deliver this for existing ground proofs now; later symbolic, constraint and higher-order proof forms extend the settled owner without changing frozen payloads.

## Acceptance

- [ ] Existing asserted and derived ground proofs independently re-derive their conclusions under the correct source/program/sort contract.
- [ ] Forged premises, a foreign source or program, incomplete negative dependencies and unestablished absence are rejected.
- [ ] Proof inspection and retention support actual consumers without exposing a payload layout that later required forms must break.
- [ ] Preserve existing valid identity laws or explicitly migrate/version changed ones; do not silently reinterpret persisted proofs.

## Release classification

Required before PurRDF v4.0: this changes existing public construction, signatures, payload ownership or exhaustive matching. Track in the API-freeze epic and v4.0 milestone; provide migration evidence to the final public-interface audit.

API freeze: https://github.com/Blackcat-Informatics/purrdf/issues/1104. Final audit: https://github.com/Blackcat-Informatics/purrdf/issues/1115.

## Dependencies and existing ownership

Depends on:

- https://github.com/Blackcat-Informatics/purrdf/issues/1141
- https://github.com/Blackcat-Informatics/purrdf/issues/1142
- https://github.com/Blackcat-Informatics/purrdf/issues/1143

## Implementation and verification

Implement complete production behavior through the shared owners; remove superseded paths within scope. Choose algorithms against semantic requirements and measured work, allocation and retained-memory behavior. No stubs, silent fallbacks or speculative duplicate mechanisms satisfy acceptance. Use focused native behavioral checks, actual affected callers and required repository hooks/gates. Host checks cover real transport/ownership obligations; do not duplicate native semantic suites on WASM. GMEOW checks consume authenticated prepared artifacts rather than building replacements.

This specification records source/planning evidence, not executed conformance or implementation qualification.

## Comments (0)

