# Task3 private output layout: independent owning diagnosis

Verdict: **PASS for the proposed task-owned output-layout correction**, not for
the failed full gate. Task3 controller86259/full-check actually failed with
make exit2; helper-census reported68 PASS/3 FAIL. Source stays frozen20e2e637c.
No source/build/ref/forge or output-root mutations were performed by this review.

## Confirmed cause and boundary

The three failures in full-check.log lines1140–1155 all occur before their policy
assertions, at TempDir::for_unit_test(). Their executable is genuinely Cargo's
helper-census test product at
/opt/purrdf-401-qualification/build/debug/build/helper-census/4bb5694e91d8322b/out/helper_census-4bb5694e91d8322b.
Neither assigned target nor build root currently has CACHEDIR.TAG. build/tmp
exists. This is an output-layout admission failure, not evidence that those
three policy tests or the new typed transfer implementation are incorrect.

The owning testkit/temp.rs lines65–73 documents nearest tagged ancestor as the
build-root law, with hard error rather than fallback. unit_test_root at195–210
uses current_exe, the nearest ancestor whose tag is a regular file, and its tmp
directory. It does not read TMPDIR or pretend an arbitrary executable is Cargo
output. The helper policy tests at612/624/654 require this legitimate scratch
root before constructing their positive/refusal fixtures. The guard is unchanged.

Task3 qualify.sh actually sets CARGO_TARGET_DIR to this target root and
CARGO_BUILD_BUILD_DIR to this build root, with the active-toolchain Cargo first
in PATH. The active SDK resolves to nightly-2026-09-14. The inherited Cargo
configuration explicitly enables build-dir-new-layout; its ordinary template
build-dir is /opt/.cargo/{workspace-path-hash}. The Stage Cargo wrapper normally
chooses separate managed target/build roots and strips custom roots, but this
authorized raw-SDK route deliberately honors the task-owned roots. Its actual
executable path and compilation log bind these directories to Cargo outputs.

A read-only normal Cargo output sample
/opt/.cargo/18/fca61885ee89f7/CACHEDIR.TAG contains the standard first line:

    Signature: 8a477f597d28d172789f06886806bc55

The precise reason this Cargo invocation did not create tags in these
pre-existing custom roots is not independently established. Do not claim a
Cargo implementation bug or that manually supplied tags were created by Cargo.
The confirmed missing-layout fact is sufficient for the proposed correction.

## Smallest legitimate correction and qualification

Root may add that valid cache-directory signature to exactly the two genuine,
task-owned output roots. Use exclusive creation, refuse unexpected existing
files/symlinks, and identify the file's remaining comment as task qualification
setup declaring Cargo output caches. Do not tag /opt or a generic ancestor,
alter testkit, change a guard/test assertion, change executable/source bytes,
move outputs, introduce a fallback, or clean any cache. Ensure build/tmp and
target/tmp are real owned directories (build/tmp already exists). Preserve the
failed command, full log and exit unchanged; retain marker bytes and output/source
readbacks separately.

This restores the actual documented root invariant: generated test products
already live below these caches. It is not a source guard bypass because it
does not relax discovery or substitute success for missing/non-Cargo evidence.
It also truthfully marks actual disposable outputs for cache-aware tools.

Smallest focused owning check, under the same resource/SDK/environment contract:

    cargo test --locked -p helper-census --bin helper-census policy::tests --jobs 8

It must execute all six current policy tests, including the three formerly
failing TempDir users, with real PASS and no zero-test acceptance. The existing
testkit temp::tests target additionally has two exact root/collision controls;
the pending full workspace gate already executes them, so a separate duplicate
run is unnecessary unless root needs earlier owning-root proof. Continue the
required whole gate without pretending the original failed invocation passed.
Prior additive semver and actual portable WASM receipts remain source-applicable:
only output-cache metadata is repaired, no shipping/source or artifact bytes
change. Whole Task3 still needs its settled required full-gate result.
