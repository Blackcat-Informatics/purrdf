# Telemetry collection correction source handoff

Status: source prepared; Cargo compilation, tests, clippy and actual production
runtime qualification NOT RUN. Root withheld local build admission while the
separate475campaign owns it. No commit/push/dispatch or other forge mutation.
Source work began only after run37746157677 actual completed FAILURE and its
comparison completed FAILURE. Original221b1ace8 measured uploads are preserved.

## Implemented existing homes

Three Rust paths now differ from221b1ace8: support/phases.rs, support/profile.rs
and tests/c_smoke.rs. No manifest, dependency, feature, Make/workflow, native
target selection, renderer or unrelated source changed. Exact source patch is
T4-telemetry-source.patch. Direct active-SDK rustfmt --check and git diff --check
actually exit0, with their source-only logs retained. No Cargo command was run.

phases::cargo_messages is the one admitted-frame parser: its explicit harness
policy preserves the collector’s existing Cargo column-zero JSON boundary;
JSON-only callers parse every nonblank line. Both paths refuse malformed/duplicate
admitted frames. phases::cdylib now selects from complete already-parsed frames;
its exact package/target/profile/features/freshness/absolute-filename and ambiguity
checks are retained verbatim. Actual C-smoke’s only adaptation invokes the shared
JSON-only parser before this same selector, preserving its strict production
boundary. There is no unused adapter lint suppression or duplicate selector.

The Cargo shim captures selected CAPI artifact plus phases::identity(path) while
the successful child owns the library, before returning to an owner that may
remove temporary scratch. This includes real `capi build` header children rather
than only the build/test timing predicate. The existing child Recorder stores
this capture and hard-fails missing/ambiguous/invalid artifacts at capture.
Collection parses each complete invocation once, matches every saved capture to
its selected frame/path and typed byte/digest identity, and preserves full raw
stdout/stderr receipts. It no longer mistakes a historical temporary path for a
currently retained runtime asset or concatenates independent child streams.

The actual C-smoke phase receipt must bind its exact selected frame AND library
identity to precisely one captured child. Parent/header artifacts cannot stand
in for it. Comparison revalidates child success across all phases, saved captures,
retained receipt/timing identities and the same C-smoke binding, while retaining
the mandatory16successful C phases. Source/config/concurrency/cache and matching
laws are unchanged. A historical capture states producer-time identity; it does
not claim transient header bytes remain on disk after legitimate cleanup.

## Written meaningful fixtures (execution pending)

One new phases fixture proves mixed Cargo/build-started/build-finished/libtest
text selects the exact artifact, while JSON-only admission still refuses that
mixed stream; malformed/duplicate admitted JSON, duplicate matching artifact,
wrong package and missing exact filename refuse. Existing strict metadata,
relative-path, UTF8 and ambiguity controls remain through the same selector.

Two new profile fixtures exercise real testkit-owned files and retained child
JSON. Independent header/parent/runtime invocations capture their own library
bytes. Removing header bytes after capture allows truthful later collection;
removing before capture refuses. Complete mixed stdout remains unchanged. The
runtime binding permits current collect/valid_receipt, while substituted valid-
shaped C-smoke digest, incomplete16phase inventory, altered retained evidence,
missing/duplicate/tampered frame/path/byte/digest bindings and malformed admitted
JSON refuse. Existing shared artifact fixture gets one src_path field and is
reused from its original test home; no second fixture formatter is introduced.

Three new test functions are written. No total test count or runtime PASS is
claimed from source inspection. Existing synthetic comparison fixture now
contains the explicit empty child capture inventory/output required by the real
schema, instead of bypassing the stronger child contract.

## Required remaining acceptance

Root independent source review precedes the admitted focused lane. Run actual
native_profile target, strict CAPI all-target clippy, actual controller build and
JSON artifact selection, live helper census and affected profile/shard/workflow/
parity/fmt gates. Actual production doc/example and nested C-smoke/header seams
must execute to establish the original caller contracts, including legitimate
test-to-dev counterfactual selection; fixture-only success is insufficient.
Then normal verified commit/push and a fresh FULL matched hosted campaign and
unchanged comparison policy remain required. Both historical failed attempts
remain failed with full uploads and no accepted speedup. No silent descope,
missing-file fallback, first/latest artifact choice, changed profile/coverage,
new helper implementation or commit bypass was introduced.
