# Preparation safety: owner assessment

This is an owner assessment, not independent review or full issue acceptance.
No child agents were used.

The original recursive lowering functions now return PreparationRefusal through
their real callers: bare-shape conformance, target checks, rule conditions/heads,
standalone node expressions and native prepared bindings. PreparedShapes retains
Result values for the original class analysis and memoized lowering. A refused
walk cannot furnish an empty catalog or a successful binding/product. Restored
products retain their carried catalog and still defer lowering until binding.

The infallible footprint callback uses the existing purrdf_stack walk scope;
its latched partial output is discarded by the fallible outer scope. No new
thread-local state or stack/toolchain override was introduced. The shared
encoder/decoder/model-walk level entry now checks physical admission as well as
its existing logical ceiling. The native preparation cause remains matchable in
ShapesError and in ShapesProductError's accessor and standard source chain.

Standalone path entry points return errors, removing their former conversion of
an invariant failure to an empty match set. Their known benchmark call sites only
adapt to Result; no benchmark or timing measurement ran. Path cloning and
inversion use one postorder implementation over purrdf_lex::walk::WorkList.
Destruction detaches children before native drop glue reaches them. Ordinary
shallow copy retains exactly four tree allocations in the tested shape and drop
adds zero allocations. No duplicate scanner, allocator, stack guard, work-list
implementation or tooling was added.

Behavior evidence: eighteen SHACL safety controls; twenty-three path cases
(including 5,000 levels on a 256 KiB stack); nine planner/class/index controls;
sixty-three rule cases; twenty-six AST codec cases; and final two preparation
controls proving all three deep built-model families refuse within an admitted
descent budget, with typed native/product causes and successful fresh retry.
The path filter incidentally matched six XPath cases too; those are not needed
to claim this unit's behavior. The initial full-stack deep-model expectation was
wrong because the shape chain fit; the fixed test preserves 2,000 levels and
starts above the canonical margin with 32 KiB available for actual descent.
Test integer-width/qualification and err-expect lints were fixed without allows.

Final strict compilation passes for all shapes targets, shared/C/Python/WASM
libraries and the CLI binary. The actual wasm32 WASM consumer compiles. A consumer
command was submitted before the shapes check ended; Cargo held it behind the
build-directory lock, so compilation remained serial. Both handles completed
successfully. Subsequent commands were started only after observing termination.
The helper census found one stale shim exemption for DrainError::new and
ShapesProductError::new after the latter gained native cause ownership. That one
registration was removed; no checker, threshold or acceptance rule changed, and
the fresh census passes with no duplicate or forbidden body. Normal hooks,
commit and push pass at 026d003f4. The source checkout is clean.

Remaining full issue work includes identity-based cyclic shapes, explicit LFP/GFP
selection, bounded shared fixed-point evaluation, non-stratified refusal, host
options/structured fields, ShEx safety, the actual sheval population, portable
execution and final gates/review/integration. This unit just closes the known
native lowering gap; no PR, merge or issue closure is claimed.
