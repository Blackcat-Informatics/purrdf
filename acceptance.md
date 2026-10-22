# Native columnar cause intake

Original intake snapshot below; final current qualification, review and
integration disposition are in validation.md and pr-comments.md.

Issue677 under558, exact stagectl owner .worktrees/677-preserve-native-content-and, branch paudley/677-preserve-native-content-and, stage .stage/preserve-native-content-and; base6f1199b1d. Parent full shipping/malformed/UTF-8/host contract remains open. Both active lanes are isolated; one owned Cargo command at a time, eight jobs/four test threads, no profile/cache/compiler exception.

Current source verifies three production erasures: writer project_read verify_all toString, reader blob insert_checked row formatting, and reconstruction freeze diagnostic formatting/storage extraction. ContentStoreError lacked the shared visitor; ContentDigest wraps the existing Digest32 inspection home. The old Graphify snapshot confirms only the ColumnarError home; current source is authoritative. Repository-bank recall supplied no relevant fact. No parent donor was edited or replayed.

Initial Clone concern was disproved before implementation: purrdf_lex::resident_clone!(RdfDiagnostic) supplies the existing trait through clone_with_memory. Issue677 was corrected and read back exactly. Existing Clone/Eq traits stay; no new box/control allocation or synthetic clone. The temporary intake body moved into this exact stage. Applicable root/canonical .baseline/.goals and COLUMNAR/v4 guidance are read; no nested crate AGENTS found. Stage is actually ignored despite open.json's schema flag.

Implementation now moves native content-store, row-bearing blob and reconstruction causes by value, visits them through original homes and exposes their actual standard source. Human prefixes remain at Display. ContentDigest delegates to Digest32's borrowed record; no digest bytes are hex-rendered for inspection. Native original diagnostics retain all public/private attachments even when storage is present. Generic source-session transport is unchanged.

First production library check is running, not runtime acceptance. Core tests now exercise actual insert_checked and verify_all failures, using a corrupt fixture solely at the store's private owner, then inspect both native digest records after store disposal. Public columnar failure/ownership/allocation cases, consumers, native/WASM goldens, strict checks, independent review and normal hooks remain required. No PR or completion claim.
