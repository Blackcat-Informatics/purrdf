# Owner source review (not independent approval)

Scope: keyed/derived hashing, streaming state and root output; all existing SIMD
chunk/parent/within-block kernels; public conformance entry points and WASM
registration. Compared the specification's domain and compression equations with
the changed computation. No upstream implementation bodies were read or copied.

Every chunk starts at the mode key; every block and parent includes its mode
flag. The compression state's fixed four IV words remain fixed in every mode.
Context derivation hashes context under DERIVE_KEY_CONTEXT, decodes the first
32 root bytes as the material key, then hashes material under the distinct flag.
The root output counter replaces the input counter, and the second eight words
xor the retained original CV. All available native kernels and both WASM builds
match official full-length answers, including the second output block.

The owned reader copies root inputs and backend, never borrows the hasher or
retains input. Range checks precede mutation; Read clamps to the specified end.
Clone copies the independent position. Debug prints position only, not root/key.
Reset preserves the mode and clears logical input/tree state. Existing unkeyed
hazmat and caller-scheduled trees explicitly retain unkeyed mode. The short x86
policy is shared with the original dispatch; forced streams retain their kernel.

Corrections found during implementation: macro fixed-IV state was restored while
mode keys were carried only into chaining state; portable unused mode arguments
were accounted for; a generic test now checks full sixteen-word compression;
a test-only explicit non-Drop drop was removed. Historical failed checks remain
in tasks and are not reported as passes. No new dependency or duplicate rounds.

Remaining acceptance: helper/assembly/generated projections, final gate and real
hosted source review/CI. This is owner judgment, not an independent review PASS.
