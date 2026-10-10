# Fallible shared control proposal

`shared-control-draft.rs` is proposed source for a narrow module at the existing
core allocation home. It is not applied, compiled, tested or reviewed as shipping
code. Rustfmt checked syntax only. No extra compiler or full gate was run.

The proposal uses one global-allocator allocation for `Header<T>` with one atomic
strong count and the sized payload. A null allocator result returns a small typed
error and drops the input exactly once. Payload initialization occurs only after
nonnull storage. Clone increments a checked count without allocation. Release
decrement and the last owner's Acquire fence precede payload destruction and
deallocation. Public callers cannot obtain a mutable payload reference, raw
pointer, weak reference, unsized coercion or detached value. The confined unsafe
body belongs in core; evaluator callers retain their unsafe prohibition.

The public `try_clone` refuses overflow without changing the count. `Clone`
panics on that astronomically large count instead of wrapping or resurrecting a
freed owner. This is distinct from construction allocator refusal, which returns
normally. The actual allocation is always nonzero, including a zero-sized T.

Integration must replace each bounded owner construction with `Shared::try_new`
and propagate its allocator refusal as `EvalError::AllocationFailed`, after
admitting `Shared::<ActualPayload>::allocation_layout().size()`. Existing
two-word Arc layout assumptions must change together with each caller. Payload
fields still precede their WorkspaceAllocation guard so allocation lifetimes
remain covered. The account's dynamic trait erasure must have one separately
priced fallible boxed concrete owner; this sized Shared does not unsize.

Migrate the actual row/bag/schema, numeric parsed cache, account, retained result,
governor and explanation controls that allocate during operational execution.
Ordinary pre-existing resident/shared input objects are not automatically new
query-owned allocations. Review every remaining operational Arc::new and
Box::new against its actual ownership path. Do not add a second control owner
while retaining the old one, or settle a fee before its covered payload dies.

Five proposed unit tests exercise null allocator refusal/input destruction,
shallow clone identity and last drop, native concurrent clone/drop, zero-sized
and over-aligned values, and checked count overflow. The native thread test is
target-gated; normal clone/drop remains tested on wasm. The null allocator test
uses the exact private constructor boundary rather than a fabricated evaluator
error. End-to-end fault injection, real admission/peak/lifetime checks and the
complete existing Rust/native/wasm gates remain necessary. No proposed test has
run; this does not establish current-head qualification.
