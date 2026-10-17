# Datatype-support boundary regression repair

Hosted CI of the native anyURI implementation failed in test(lib), job 117356063207, run 39018704037: 717 passed, one failed, one report-only ignored. The sole failure is owl_dl::data::tests::an_unmodelled_datatype_is_reported_rather_than_guessed, whose unsupported fixture is now the supported xsd:anyURI primitive. The exact failure is an empty boundary list versus the formerly expected DataRange boundary. The native implementation and its source pin are not changed by this test-only repair.

## Task 1: Repair the boundary contract

Replace the stale unsupported fixture with owl:real, which is genuinely outside the native XSD model. Keep an explicit anyURI row proving the promoted primitive now has no DataRange boundary. Both rows must remain consistent. Do not delete the negative capability coverage, force an unsupported boundary onto the supported primitive, change the engine, or relax assertions.

Run the complete affected native entailment data-test module and normal configured hooks. Publish and merge the single-file repair with ghprsq under existing user authority. Preserve pending hosted CI as pending and retain the original native implementation qualification at refs/ghprsq/pr/573/stage. The paired downstream GMEOW deletion remains in progress on its existing worktree; issue 572 stays open until it lands.

## Task 2: Repair finite length-space exhaustion

Review found that the inherited character-space exception predicate assumed a positive-length character space could not be exhausted by a finite enumeration. Replace this false assumption with a mathematically exact comparison, saturating only at the queried exception count plus one. XML characters and octets have finite alphabets; collapsed anyURI values additionally prohibit leading/trailing and adjacent XML spaces. A threshold test must witness the shared XML character census and exact boundary counts without producing a million-value corpus. Include maximal length/count termination and the octet overflow boundary. Run the full affected XSD crate suite, all-target warnings-as-errors clippy, actual WASM anyURI replay, and the downstream native datatype module after this production change. GMEOW must adopt the final fixed source pin and refresh its tiny attestations; do not retain the inherited predicate as a fallback.
