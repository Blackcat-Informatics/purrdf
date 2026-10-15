# Representative growth checkpoint

The original control reproduced 4096 replacements for 4096 imported identities.
Commit `dc9741fcc` changes the sole incremental representative caller to the
existing checked amortized reserve_for_push home. Exact reservations throughout
the workspace remain unchanged. No duplicate helper or allocation path is added.

Actual native owner regression passes: at most 12 capacity transitions and fewer
than 8192 copied entries, with original retained-term ownership intact. The
existing public identity control passes natively and on actual WASM, importing
4096 values with under 16 MiB cumulative requested allocation, zero final retained
bytes and peak within the caller's live-byte limit. Memoized zero-allocation hits
and sticky failed import pass. Warning-denied all-target Clippy passes after
explicit last-use guard release. Normal commit signing/hooks passed.

The unsuccessful portable unit-harness invocation remains recorded; the correct
existing portable integration runner passes without a tooling change. This is
an owner assessment, not an independent review or complete issue qualification.
The full persisted consumer, LIMIT consumption, publish admission and image byte
breakdown remain active tasks in plan.md. No PR, merge or closure is claimed.
