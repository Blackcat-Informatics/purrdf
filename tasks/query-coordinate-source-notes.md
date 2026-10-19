# Verified query-coordinate seams

No clock/seed implementation is claimed yet. These observations identify the
production paths to change after the published historical unit.

- QueryOptions is non-exhaustive and Copy. Borrowed coordinate inputs can retain
  that existing public construction contract. EMPTY must keep live defaults.
- EvalCtx and its now/rng_state fields are public. Preserve the existing field
  types; avoid a replacement representation that breaks current callers.
- Native DateTime has private validated fields and an exact Decimal seconds
  component. Caller values can own heap storage. Existing owned_heap_bytes
  reports that storage; cloning requires admission before allocation and a
  retained owner through root, worker and nested-context lifetimes.
- fork_with_scratch and the function-child constructor already return fallible
  results. Their current now.clone sites require explicit review for wide inputs.
- apply_query_options runs after context construction. Overwriting fields there
  alone would still perform the old OS clock/entropy reads. Select explicit
  inputs before those defaults are invoked.
- The admitted call cursor creates a separate FilterContext in engine.rs and
  pins NOW there. Its per-pull contexts must share the selected input as well.
- Evaluation identity, prepared reuse, rule callers and shipped host options
  remain part of Task 2. No narrow root-only setter completes it.
