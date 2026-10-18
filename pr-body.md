## Problem and repair

Prepared DL proof contexts and retrieval profiles independently implemented the same owned-or-borrowed accessor, causing main's helper-hygiene job to fail. Both now use one inline immutable carrier in the existing common core crate. The private duplicate enums and accessors are removed.

The carrier requires no Clone or ToOwned, allocates nothing for borrowing, and preserves normal ownership, lifetime and drop behavior. The helper ledger enforces its single home. Existing capacity proofs retain their sinks, authorities and invariant reasons; their constructor-set references are renewed only after reviewing the newly visible module and verifying all prior constructor sources and named authorities are unchanged. No census or proof policy changes.

## Validation

Passed: strict all-target Clippy for core, entailment and retrieval; two new carrier controls within the six-test ownership selection; four independently prepared proof-context tests; all 114 retrieval library tests and 21 public stage-boundary tests; and the actual borrowed-context lifetime compile-fail test. The normal commit and push hooks passed. The complete helper-hygiene target passed, including the shared-helper and domain registries and their self-tests.

This repair is separate from the unfinished causal dependency graph in #596. No full-workspace or WASM CI result is claimed.

Closes #597.
