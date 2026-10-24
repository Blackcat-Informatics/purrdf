# ShEx parser safety: owner assessment

This is an owner assessment, not independent review or full issue completion.

The real ShExC/ShExJ entry points check physical stack admission before work;
the ShExC recursive-descent entry and both ShExJ recursive readers check again
at each descent. They all call one typed admission function over the existing
purrdf_stack::is_low home. The existing logical depth limits are unchanged.
The error records the refused construct as a public native enum field, with no
invented byte offset, empty schema fallback or swallowed error. ShexError was
already non_exhaustive, so the new resource variant preserves its established
matching contract. No existing Shape field or Constraint variant is included in
this commit. Those separate uncommitted model changes still fail compatibility.

The existing first-party stack dependency is promoted from dev to runtime and
the exact layer edge is registered; no external package or build/cache/toolchain
setting is added. Documentation states the actual resource failure.

Two public integration cases pass: both exhausted entry points retain the native
refusal and recover after reserve release; a 4,000-level valid compact expression
is refused safely on 256 KiB. The latter checks the exact logical-depth reason
if it reaches that ceiling instead of physical margin refusal, so an unrelated
syntax error cannot satisfy the regression. The private-reader case exercises
both ShExJ reader boundaries and healthy retries. Ten existing compact-parser
and eleven JSON-reader cases pass, including the 128-container boundary.
Strict all-target ShEx compilation, actual wasm32 library compilation and the
dependency layer census pass. No timing measurement ran. The portable result is
compilation, not execution.

The normal commit hook passes at 4a71eeec1. Publication is tracked separately in
validation.md. This closes only parser stack admission: ShEx runtime/preparation
safety, explicit SHACL recursive semantics, host/product integration, complete
corpus, final gate, actual independent review and delivery remain unfinished.
