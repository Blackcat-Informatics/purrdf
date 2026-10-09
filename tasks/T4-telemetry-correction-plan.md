# Bounded telemetry caller correction design

Status: source-only design; no measured-source edit, build, push or dispatch.
Current corrected attempt37746157677 remains active; retain it through its actual
terminal. Root owns implementation after that terminal and serialized validation.

## Two distinct observed failures

after-doc job113208296102: config/source/target/native phases all SUCCESS;
telemetry fails `JSON byte 0: expected a JSON value`. Exact upload11536587458,
arm-after-doc.zip and job-after-doc.log are retained under the corrected-run
evidence directory. Three child receipts record successful workspace doc,
workspace examples build and selected example tests, with three attributed HTML
reports. Mixed stdout includes Cargo JSON then actual libtest text. collect filters
JSON frames, but invokes phases::cdylib on the entire stdout; that strict selector
correctly refuses ordinary harness text. This is a caller-boundary bug.

before-capi job113208296197: config/source/target/native phases all SUCCESS;
telemetry fails `actual selected C library is missing or ambiguous`. Distinct
upload and job log retained as arm-before-capi.zip/job-before-capi.log. First
sorted Cargo child is the actual header regeneration command:
`capi build -p purrdf-capi --target-dir /tmp/purrdf-header-44kjwfa4/build
--message-format=json`. Its exact single CAPI artifact lists the temporary
target's libpurrdf.so; there is no second matching CAPI frame in that child.
scripts/capi-header.py owns this TemporaryDirectory and exits its context before
outer collection. Thus the generic error's owning computation is post-lifetime
path existence, not an ambiguous selector or failed C runtime. This diagnosis
follows exact recorded argv/frames and owning cleanup source, not the message
alone. No remote live-file observation is claimed.

The separate actual C-smoke phase receipt records a different exact selected
artifact at $ARM/target/debug/libpurrdf.so, bytes45245880,
BLAKE3f3273a9fa2dfab8265962b8e683f8328c84e70d4751a954751e7481a545b1f16,
fresh=true, O3/assertions/overflow on. All16Csmoke phases, including actual object
compile/link/runtime and projection validation, succeeded. Parent cargo-test
also reports its own build-root artifact; these distinct invocation records
must not be merged into one ambiguous stream or picked by first/latest.

## Coherent correction in existing homes

1. Keep the strict JSON-only phases::cdylib entry point for C-smoke. Extract its
   exact package/target/profile/features/freshness/filename selection computation
   to a borrowed parsed-message entry in the same phases home. Both entry points
   call that sole selector body. The JSON-only adapter still refuses non-JSON,
   duplicate members, missing/wrong identities and ambiguous filenames.
2. In profile, gather admitted Cargo frames from each retained child stdout ONCE
   before artifact processing. Preserve its existing deliberate mixed-output
   boundary (Cargo-prefixed frames versus harness text), parse every admitted
   frame strictly, and pass the complete parsed frame collection to the phases
   selector. Do not reparse the full mixed stdout or create another inconsistent
   starts-with filter at the cdylib call site. Reuse this same frame home in any
   child-completion capture below. Raw command stdout/stderr remain verbatim in
   the original receipt; artifacts retain complete profile/features/freshness.
3. Bind artifact liveness to the actual producing child lifetime. In cargo_shim_at,
   after successful real child completion and before returning control to a caller
   that may drop private scratch, use the same admitted frames and exact selector
   to capture the selected CAPI artifact plus phases::identity of its library.
   Record invocation-bound selected frame/path/byte count/BLAKE3 in that child's
   existing Recorder context. Missing/ambiguous/invalid artifacts at this point
   HARD FAIL. Capture header-helper output too: its original argv begins `capi`,
   so it cannot be accidentally excluded by the build/test timing-only predicate.
   No new timing mode or profile/target override is needed.
4. Collection validates the saved child-time binding against that SAME child’s
   admitted selector result and retained receipt identity. It must not require
   every historical temporary library path survive a successful owning cleanup,
   or flatten different child streams before ambiguity checks. The selected C
   library actually exercised remains bound by the existing C-smoke artifact and
   library identity/16phase receipt; cross-check this exact frame/path binding,
   rather than pretending every parent/header artifact was that runtime library.
   Unchanged saved receipts/timing bytes remain independently checked. Any absent,
   changed, duplicate or mismatched child-time binding is refusal, not a fallback.

The child-time binding is the recommended concrete design: deleting the late
is_file check without evidence would hide missing artifact admission, while
keeping temporary artifacts forever would change an original production caller’s
ownership and cleanup. Do not choose first/latest across legitimate independent
invocations, add ledger exemptions, loosen the strict selector or silent-skip
malformed frames. Exact schema/name choices are root’s implementation judgment;
there must be one frame-selection home and one exact artifact-selection home.

## Required meaningful fixtures and qualification

Existing homes: phases tests for strict selector; profile tests for collection
and child receipts; native_profile harness executes both. Use testkit-owned
scratch and the platform DLL filename. Reuse retained real Cargo frame structure,
including fresh/profile/overflow/features/filenames fields; no fabricated passing
measurement or real repository/index mutation.

* Mixed real-structure Cargo frames plus libtest/doc lines: exact selected frame
  and all artifacts retained, raw stdout unchanged, collector passes. Include
  unrelated package and build-started/build-finished messages.
* Malformed admitted JSON and duplicate fields refuse even with harness text.
  Missing/non-object/wrong-profile/features/path metadata retain strict refusal.
* Duplicate exact cdylib candidates in ONE child refuse; distinct header/parent/
  C-smoke child invocations remain separate and bind their own selected identity.
  Wrong package, no exact filename, relative path and missing library AT CAPTURE
  refuse; never first/latest or guessed target paths.
* Actual owned scratch library exists at child-time capture, then normal owning
  removal occurs: retained binding passes later collection. Removing before
  capture refuses. Tampered/missing/duplicate binding, wrong selected frame/path,
  changed retained child receipt or timing identity refuses.
* Nested C-smoke selected artifact/library frame exactness versus parent and
  temporary header frames: correct16phase/runtime identity passes; substituted
  artifact/path/hash or incomplete C phase inventory refuses. Preserve the actual
  test-to-dev counterfactual’s independent legitimate child records.

After source settles and root admits the lane: cargo test --locked --jobs8
-p purrdf-capi --test native_profile (discover actual new count, not assume25);
strict CAPI all-target clippy; actual selected native_ci_profile controller build;
live native/shared-helper census; affected workflow/shard/profile/parity/fmt gates.
Run meaningful actual production doc/example+C smoke seams only with admission;
fixture PASS alone does not establish those nested real Cargo output contracts.
Then normal verified commit/push and a fresh FULL matched hosted attempt, not
partial reuse of failed arm uploads. Current failures remain visible and the
completed future comparison must pass its existing unchanged matching laws
before measured acceptance. No measurement/speedup is claimed by this design.
