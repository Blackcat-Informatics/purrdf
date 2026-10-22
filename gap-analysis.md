# Fallible evidence implementation review

This is an implementer review, not independent approval. Scope: all current
evaluator, execution, workspace and retrieval changes, the shared page-provider
fixture, public evidence matrix and real-producer controls.

The answer, status and interned doors share query_interned_input_admitted with
the original resident API. Prepared witnessed execution shares
execute_fallible_input and resets after final failure. Source errors retain their
native error and final operational receipt. No dataset conversion or alternate
evaluator was introduced.

Retrieval retains the same plan, compilation, execute_bound and fusion bodies.
ReadOperation retains the original typed QueryWorkspace across awaited execution
and deferred exclusion lookups. A capability alone does not retain a typed source
failure owner: its original with_capability path has no typed account. The new
composition keeps the actual account; unbounded fallible sources have an admitted
shared direct-error slot. Both paths keep the first cause without formatting it.
The enclosing with_fallible_read boundary uses the existing preflight, reporting,
execution admission and finish_fallible_read machinery.

Actual tests cover resident/SegmentedStackQuery score and evidence parity,
semantic unknown neighbors, a provider fault after candidate evaluation starts,
deferred exclusion failure even after its signal is caught, and descriptor drift
after successful stream reads. The existing verdict matrix now also runs over
real paged fallible storage for all four query forms, signed information, early
stopping before a poisonous suffix, summaries and unknown closed absence.

Further adversarial review found a gap in direct evidence entry points over an
unbounded source returning a non-latching point-read failure. The actual public
interned entry returned Query with a rendered source diagnostic instead of its
original operational cause. The new public-entry regression reproduces this
before correction. Direct-error capture has moved into the shared
reserve_fallible_execution boundary, so all evidence doors and the asynchronous
composition receive it; the composition no longer captures a separate owner.
Capture is idempotent and retains the same first cause across repeated admission.
The corrected suite passes all 66 ownership cases, including actual calls through
all four public evidence doors preserving the original cause without cloning it.
All 62 native composition cases and strict affected all-target checking also pass
after correction. All 45 cases on each scalar and SIMD WASM now also pass on the
corrected source. This gap is fixed and locally qualified; the complete delivery
still needs its helper/hooks, hosted gates and independent review.

Before this correction, 62 native composition cases, 65 ownership cases, strict
affected all-target checking and 45 cases on each scalar and SIMD WASM passed.
Portable qualification now covers the correction. Focused helper checks and
normal hooks remain; broad current-head hosted gates and substantive independent
review must pass before merge. No PR, merge or issue closure is claimed. See
validation.md for commands, failures and their actual dispositions.
