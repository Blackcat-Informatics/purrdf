# Main integration assessment before the next matched campaign

Status: READ-ONLY assessment complete. Synchronization is required before final
profiling qualification. No merge, index/source mutation, Cargo command, build,
test, push or dispatch was performed for this assessment.

## Exact inputs and preservation

- Branch HEAD: `221b1ace8bd30e010dba6b9d732cfc9ff576a0ed`.
- Observed `origin/main`: `5384882d65750ee22bddfeeac473095ab9c3db03`.
- Merge base: `6273b6173f3b3d98f49629a162ba88da8bc26477`.
- Uncommitted correction: exactly `crates/rdf-capi/tests/c_smoke.rs`,
  `crates/rdf-capi/tests/support/phases.rs`, and
  `crates/rdf-capi/tests/support/profile.rs`. None is changed on incoming main
  relative to the merge base. The three corrections therefore have no direct
  incoming path overlap; this is not a claim that their consumers are unchanged.
- Preserved `tasks/T4-telemetry-source.patch`, SHA-256
  `873b32d4d076b813a7509ce667d0a6dca1142ebfea0258914392e4f8ab336152`.
  The patch was not altered or reapplied.

`git merge-tree --trivial-merge <merge-base> HEAD origin/main` exited 0 and its
complete textual output is retained in `tasks/T4-main538-merge-tree.txt`.
This read-only legacy prediction does not execute the eventual ordinary merge
or certify the modern merge strategy's result. Its conflict markers identify
one concrete conflict, below; exit 0 is not treated as a conflict-free verdict.

## Predicted merge decisions

The sole textual conflict is `crates/rdf-capi/Cargo.toml`: both sides insert
`purrdf-testkit` at the same dev-dependency position, and the branch also inserts
the `native_ci_profile` example. Resolve by keeping ONE workspace testkit
dependency, the incoming explanatory comment if useful, and the entire example
declaration including `test = false`. Dropping that example breaks the actual
hosted controller build; retaining duplicate TOML keys breaks the manifest.

Two further files are marked changed on both sides without conflict markers:
`Cargo.lock` and `.github/workflows/ci.yaml`. The textual prediction retains
main's new helper-census `purrdf-jsonschema` and Datalog `purrdf-alloc-probe`
edges alongside the branch's existing CAPI testkit edge, and replaces the
workspace's legacy Python glossary call with the native helper call. Preserve
the branch's profiling workflow/shard changes AND the incoming native glossary
call. The predicted Makefile merge similarly replaces only the two glossary
invocations, retaining the branch's profiling/shard/CAPI routes. Actual lock,
workflow, profile and caller gates must adjudicate the resolved composition.

## Material shared interactions

Incoming main changes 346 paths (65,324 inserted / 3,821 deleted lines in this
observed comparison). No changes are reported for root `Cargo.toml`,
`rust-toolchain.toml`, `.cargo`, `scripts/capi-header.py`, the shard checker or
build-profile checker. The workspace's relevant target/dependency inventory
still changes substantially:

- CLI gains `rules_campaign` and `rules_alloc` examples with `test = false`.
  The doc shard builds workspace examples, so both enter actual compile work
  without being silently added to the explicitly selected example test route.
- Datalog gains `rules_runtime` (`harness = false`),
  `factor_allocation`, and `negative_guard_admission`, plus the alloc-probe
  dev edge. Their names enter the existing integration wildcard routes:
  `factor_allocation`/`negative_guard_admission` in integration-2 and
  `rules_runtime` in integration-3. The harness-free runtime also makes the
  corrected mixed Cargo/harness frame boundary directly relevant.
- CAPI gains `xpath_regex` in integration-4 and testkit-backed regex-law
  tests. Helper-census gains `glossary_hook_probe` (`test = false`) and the
  bounded JSON Schema dependency. Its explicit example is built by the doc
  shard but its resource-heavy main is not a normal example test invocation.
- Main replaces the Datalog factor planner/partial-delta matching path,
  affected entailment consumers and owned host goldens. The current
  `crates/validate/tests/fixtures/regime-boundary.vectors` is changed. C-smoke
  already reads that shared fixture directly: retain that consumer, not the
  old pinned bytes or a separate regenerated local copy.
- Main adds fourteen XPath C ABI entry points and append-only status 13,
  together with current committed header, signature snapshot, Rust ABI tests,
  and actual `smoke.c` law/resource assertions. The existing Rust C-smoke
  wrapper is unchanged on main, but its generated header, compiled C object,
  linked library and runtime/projection work all change. This is substantive
  production acceptance, not just a new compile target.
- Current CLI, core XPath/kernel, evaluator governor/allocation, SHACL/ShEx,
  validate and other language seams change native compilation and runtime work.
  No existing C ABI prototype is replaced by the new twins, but neither library
  bytes nor workload identities can be assumed identical to the old campaign.
- Native glossary CI/hook callers and the deleted legacy Python gate must remain
  coherent with the incoming helper source, catalogue and staged-index semantics.

The profiling implementation obtains `cargo metadata --locked --no-deps` at
each arm admission and hashes all tracked/unignored source paths, not a stale
hardcoded target list. Warm admission compares the full normalized identity.
The hosted nested-dev counterfactual clones the SAME `GITHUB_SHA` and changes
only the admitted C-smoke nested profile; it does not use old main. The strict
CAPI selector matches package/target/cdylib/profile/features/freshness and exact
absolute artifact filenames, not a ABI-version or old library hash constant.
The new telemetry patch binds each child's library while alive and matches the
actual C-smoke-selected frame/library, preserving the original 16 C phase
structure. Source reading finds no new incoming semantic incompatibility in
these interfaces, but no runtime success is inferred from that observation.

## Required next acceptance and evidence limits

1. Root ordinarily synchronizes this isolated branch with the observed current
   main, preserving the exact dirty correction and resolving the manifest by
   evaluating both sides. Verify the three-file correction survived unchanged
   except any explicitly reviewed necessary adjustment.
2. Under the admitted single 8-job lane, discover and run the actual focused
   profiling fixtures, strict CAPI all-target clippy, current helper census,
   formatting, workflow/shard/profile/caller checks, and rebuild the actual
   controller through Cargo JSON artifact selection. Run bounded actual
   mixed doc/example and CAPI header/nested test-to-dev seams. These checks
   qualify new source, not new timing claims; all are currently UNRUN for this
   correction and future merged composition.
3. Commit/push only through normal verification; then run a fresh COMPLETE
   matched hosted campaign on that committed composition. All twelve arms,
   each admitted cold/warm pair, current target/config/source/tool identities,
   invocation-bound captures and final comparison must pass. Do not rerun only
   failed jobs or combine uploads from different attempts.

The failed `37746157677` campaign remains preserved against `221b1ace8`:
successful native/C-smoke commands remain valid observations of those bytes;
eight collector failures and final comparison refusal remain failures. Its four
successful pairs cannot qualify timings for a new merge composition. Earlier
main-foundation tests/hosted qualification can justify avoiding an unrelated
blanket local full gate, but cannot replace current profiling fixtures, actual
C/header consumers, inventory/profile gates or the complete new campaign.
Task4 numerical/performance acceptance remains NOT MET. This report recommends
synchronization and scoped qualification, not another full local suite merely
because the branch is behind.
