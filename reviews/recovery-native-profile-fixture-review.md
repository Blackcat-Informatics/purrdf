# Native-profile fixture repair: bounded source review

Reviewed the fixture/provisioning delta in `crates/rdf-capi/tests/support/hosted.rs`,
`profile.rs` and `.github/workflows/ci.yaml`. Other existing workflow geographic
registration changes are outside this follow-up. No builds or tests were run.

The initial no-material-defect conclusion below is withdrawn. Rebuilt R2 runtime
reported 32 PASS / 4 FAIL: all four reclaim fixtures fail through a transitive
request validator that this initial review missed. The configuration absence
test passed. See the corrective diagnosis following the initial observations.

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

## Corrective diagnosis after rebuilt execution

The actual failure chain is `hosted::tests::retained_case:1219` →
`hosted::completed_case:752` → `profile::completed_pair:1122` →
`profile::validate_request:264–273`. The last function requires `/opt`, so a
canonical repository-owned fixture still fails before reclamation is exercised.
`validate_pair` is a different comparison-identity function and is not on this
failure path. The earlier review inspected containment but failed to follow the
completed receipt through this additional request gate.

The smallest coherent correction is to separate the existing request's shared
structural validation from admission to execute a profiling arm. Keep one body
for lane, concurrency, warmth/previous, nonempty declarations, absolute source/
Cargo/directory paths and refusal of parent components. Keep `validate_request`
as that validation plus the existing `/opt` execution-placement rule, and keep
`run` calling it before any directory creation. Its subsequent canonical `/opt`
and outside-source check stays unchanged. `completed_pair` can use the structural
request body followed by its existing exact case directory, lane, warmth,
previous-cold-path, complete artifact proof and unchanged warm identity checks.
No test-only bypass or fictitious `/opt` request should be introduced.

This does not grant hosted reclamation an arbitrary target. Its public entry
`hosted::reclaim` obtains an allowlisted case from `selection`, derives the fixed
`/opt/purrdf-native-profile/<numeric-run>-<numeric-attempt>` campaign itself,
requires real canonical campaign/resources directories and an admitted upload
identity, then calls the private reclamation body on that case. The private body
requires real canonical case/target/build directories and exact unchanged proof
before deletion. All of these boundaries must remain. The existing execution
admission test must still reject `/tmp/arm` and parent components, while the four
actual local reclaim tests must pass through complete receipt validation.

History separates the two concerns: arm placement originates in `eb875cb8c`;
`40de8b040` later moved fixture/configuration tests to `/opt` and added provisioning
documentation. `CONTRIBUTING.md:45–60` still instructs sudo provisioning, claims CI
does it and says tests never substitute a path. That paragraph is now stale and
must describe owned testkit fixtures and the read-only root absence query after
the coherent repair. No execution-placement or campaign acceptance change is
needed for that documentation correction.

This follow-up used source and parent-reported runtime evidence only. No build,
test or shipping-source edit was performed. The four failures remain open until
the actual corrected implementation is rebuilt and executed.

## Source follow-up on the validation split

The written production split implements the correction coherently:
`validate_request:238` calls `validate_request_fields` plus `/opt` admission;
`run:296` still uses it before any arm directory creation and retains the
canonical `/opt`/outside-source check. `completed_pair:1133` calls the shared
fields body, then retains all exact case/lane/warmth/previous/proof checks. A
caller search found no alternate execution entry through the fields-only body.
The hosted campaign and private reclamation boundaries described above remain.
The CONTRIBUTING paragraph now accurately distinguishes local fixtures from
measured arm admission. No material defect found in those production/doc changes.

One new regression assertion is incorrect: `profile.rs:2678–2680` assumes the
owned TempDir's arm is outside `/opt` and therefore execution-inadmissible.
`temp_dir!` uses `CARGO_TARGET_TMPDIR`; the hosted campaign intentionally builds
under `/opt/purrdf-native-profile/.../target`, so its owned fixture is under
`/opt` and the added refusal assertion fails. Keep the owned fixture's fields
validation, and establish the fields-versus-execution distinction on the
existing fixed `/tmp/arm` request (fields pass; execution refuses). Do not make
the assertion depend on a developer/runner's Cargo target placement. This test
finding remains open pending correction. Source review only; no execution.

Final readback: the test now admits fields and refuses execution for the fixed
`/tmp/arm` path; the owned TempDir arm checks only fields admission. The portability
finding is closed at source. No remaining material source finding in this bounded
repair. The original four runtime failures still require a rebuilt passing
native_profile target; this source verdict does not replace that acceptance.
