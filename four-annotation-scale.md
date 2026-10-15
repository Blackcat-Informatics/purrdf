# Supplemental four-annotation persisted qualification

The primary committed regression and its 256 MiB admission criterion are
unchanged. This additional final-contract check varies only its fixture to four
distinct annotations per claim, comparing the same independent/resident answers,
LIMIT and complete COUNT at 2000/200000 claims and all three fitting layouts.
Production code is unchanged. The original test source was clean before the
probe, retained as four-annotation-scale-original.rs, and restored by an EXIT
trap after each invocation. The final working-tree diff is empty.

## Actual outcomes

The 200000-claim, 4096-byte/32-record image has exactly 600263 terms and
190832640 bytes, matching the population and image size reported in the issue.
It contains 1000000 virtual statements. Present bound keys return five rows
and absent keys zero, checked independently and against resident execution.
All present queries touch eight terms and absent queries touch zero. Across
all three layouts, provider reads are at most 345600 bytes. These keyed queries
retain the 256 MiB budget and configured 4096 cache entries; no acceptance bound
was relaxed.

The first supplemental invocation failed at the full million-row COUNT unwrap:
its next reservation required 295213404 bytes against 268435456. The production
outcome is typed Residency refusal before allocation, not a wrong answer.
This failed invocation is retained as four-annotation-scale.log and exit 101.
It is not relabeled as a passing test.

The distinct positive invocation gives only the full million-row COUNT an
explicit 512 MiB budget. Its existing LIMIT/keyed budgets and read/work bounds
are unchanged. Independent COUNT is 1000000 and matches resident execution at
every layout. Observed source-admission peaks are 343405212, 343405212 and
318370460 bytes, respectively. All assertions pass, exit 0. See
four-annotation-scale-positive.log and its terminal receipt. This characterizes
the larger full scan's resource requirement; the original primary control still
passes unchanged at 256 MiB. No production default, governor, error path or
acceptance rule was modified to make the larger scan complete.

## Reproduction and scope

Start from publication 1f80189d9 with the test file clean. Apply the selected
captured patch with git apply -p0: four-annotation-scale.patch reproduces the
first case; four-annotation-scale-positive.patch adds the explicit full-COUNT
budget and keyed evidence output. Use the standard installed Cargo:

```
CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test --locked \
  -p purrdf-sparql-eval --test segmented_query \
  persisted_reifier_limit_bounds_touched_terms_and_full_count_matches \
  -- --exact --nocapture
```

Both executions use the existing optimized test profile, four build jobs,
four test threads and a 16 GiB native virtual-memory ceiling. Restore the
original source afterward. These are native file-backed correctness/read/
ownership measurements, not timing measurements or new performance claims.
The existing native/WASM four-annotation history controls retain their own scope.

The implementation, original complete local gate, strict checks and hosted
candidate remain unchanged. No extra whole-suite run or implementation branch
was introduced, and no required behavior is moved to a follow-up issue.
