# Issue #1117: v4 API: expose typed loss-ledger completeness and soundness failures

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement

## Body

Release gate: #1104. Final compatibility audit: #1115. Original structured-error contract: #558.

## Verified gap

At main `05d1e3550752d8f185710da0b00459651955d9f1`, public [check_ledger_complete](https://github.com/Blackcat-Informatics/purrdf/blob/05d1e3550752d8f185710da0b00459651955d9f1/crates/rdf-core/src/loss.rs#L2321) formats missing/unexpected code lists into `String`. Public [check_ledger_sound](https://github.com/Blackcat-Informatics/purrdf/blob/05d1e3550752d8f185710da0b00459651955d9f1/crates/rdf-core/src/loss.rs#L2363) renders the already-typed `LossSoundnessError` returned by `check_ledger_sound_in`.

These helpers are re-exported through Core/RDF/facade. They have real schema-import and LinkML production callers in addition to native tests and shipped examples. Their testing-oriented names do not make the public signatures exempt from semver.

## Required delivery

- Finalize a typed extensible completeness refusal retaining the actual missing and unexpected code sets as structured lists, with deterministic ordering and current set/duplicate semantics.
- Return the native typed soundness refusal through the built-in-profile helper instead of rendering it. Reuse the existing explicit-profile implementation; preserve original pair/code fields and first offending entry.
- Settle owned/borrowed result contracts now and document any necessary lifetime/ownership changes. Keep panicking assert helpers as explicit presentation/invariant boundaries over the same typed checks.
- Migrate real schema-import/LinkML callers, native tests and shipped examples. Preserve native causes in caller error owners rather than wrapping the rendered message in another typed error.
- Preserve current success behavior, empty-ledger/profile rules and human diagnostics at presentation boundaries. Add no new loss-profile policy or serialization format.

## Acceptance

- [ ] Actual missing/unexpected-code failures expose complete original lists through generic structured inspection.
- [ ] Pair mismatch and unregistered-code failures retain original native fields through both explicit- and built-in-profile checks and their production callers.
- [ ] Existing valid, duplicate/order, empty-ledger and neighboring refusal cases retain their semantics and presentation.
- [ ] Affected production callers/examples compile with required warnings/profile; focused native checks pass and migration guidance covers changed public signatures and ownership.

This is an additional pre-v4 child discovered by #1115. The remaining compatible #558 producer/visitor work stays open. Source inspection is not runtime qualification; no broad suite, new test dependency or duplicate WASM semantics execution is required by this child.



## Comments (0)

