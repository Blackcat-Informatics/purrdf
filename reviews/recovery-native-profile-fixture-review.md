# Native-profile fixture repair: bounded source review

Reviewed the fixture/provisioning delta in `crates/rdf-capi/tests/support/hosted.rs`,
`profile.rs` and `.github/workflows/ci.yaml`. Other existing workflow geographic
registration changes are outside this follow-up. No builds or tests were run.

No material source defect found in the repair.

- The four reclaim tests now use the existing `temp_dir!` home and canonicalize
  the temporary root before forming positive case/cache paths (`hosted.rs:1234`,
  `1279`, `1339`, `1368`). This preserves the production canonical-real-directory
  requirement even when the qualification target path contains a symlink.
  The original changed/missing proof, wrong-lane, preserved sibling, malformed
  cache, `..`, aliased case and symlinked foreign-target refusal assertions remain.
  The owning TempDir stays live until the case completes. No production path
  validation or reclamation computation changed.
- The SDK configuration test (`profile.rs:1928–1968`) queries absence with a
  read-only filesystem-root working directory and an isolated Cargo home under
  its repository-owned fixture. Configured and malformed queries retain the
  fixture working directory. The real SDK executable, environment clearing,
  jobs=4 assertion, private token exclusion and actionable malformed-response
  refusal remain. Nothing in this delta creates or changes a root configuration.
- Both obsolete `/opt/purrdf-native-profile-tests` provisioning sites and their
  fixture-only wiring requirements were removed coherently. The actual hosted
  campaign directory and its independent measurement protocol remain unchanged.
- `SETUP_START` now directly checks exactly one containing step, unconditional
  execution, order before the shared controller build, and timestamp order before
  resource-directory creation, timestamp copying and initial disk observation
  (`hosted.rs:369–417`). Missing, conditional and late step mutations plus missing
  timestamp mutation remain negative controls (`1037–1076`). The current workflow
  has the matching timestamp and commands in their required order. Existing
  setup completion, case ordering and artifact/reclamation checks remain.

This is source review only. The currently running collection uses the previously
compiled test binary: its five PermissionDenied failures are pre-repair evidence,
not a result for this source. The rebuilt native_profile target and applicable
mandatory gate must establish runtime acceptance.
