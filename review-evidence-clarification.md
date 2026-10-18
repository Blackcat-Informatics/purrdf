The six-test count comes from `cargo test --locked -p purrdf-core --lib ownership`. There are exactly two new carrier tests; the broad name filter also selects four existing ownership tests. All six passed:

```text
ownership::tests::borrowed_nonclone_payload_is_the_original_and_outlives_the_carrier
ownership::tests::owned_nonclone_payload_moves_and_drops_once
small::tests::every_ownership_path_drops_each_element_once
diagnostic::tests::shared_attachments_preserve_original_reporting_and_admitted_ownership
xsd_regex::xpath::cache::tests::shared_ownership_reuses_current_admission_without_holding_the_cache
small::tests::ownership_matches_vec
```

The native before/after constructor audit ran `cargo run -q --locked -p helper-census -- --root <checkout> --index` against the unchanged base checkout and this issue worktree. Projecting the native results to source-set comparison fields gives:

```text
baseline constructor sources: 164
final constructor sources: 165
added: crates/rdf-core/src/ownership.rs
removed: none
changed existing constructor sources: none
```

All 164 original constructor-source identities match; every named proof authority is unchanged. The complete ownership module was reviewed: its Debug derives conservatively place it in the set, while the carrier and tests create no term IDs or capacity indexes. The later doc attributes change its recorded source identity, so its constructor reference was renewed again after the documentation-only review. Every other proof field remains unchanged.

Raw before/after native index projections, the source-set comparison, and the full local logs are selected for the structured merge evidence. The complete final `make helpers-hygiene` target passed, including both census and domain registry stages and their self-tests. This clarifies the exact local execution evidence; it does not claim independent rerunning or a full hosted matrix pass. The PR and final notes now distinguish the two new tests from all six filter matches.
