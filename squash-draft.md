Run RDFC n-degree search on heap frames

Closes #710

Deep blank-node searches previously grew the native call stack for each child
search. The sole production traversal now suspends parents in owned heap frames
and resumes them with the child's issuer and hash. Related-hash and permutation
order, both pruning points, first-wins ties, canonical bytes, search charging and
stop checkpoints remain unchanged. No recursive fallback is retained.

The native 256-node chain runs on a 128 KiB stack and pins issuer order and
exactly 766 work units. All 78 selected native canonicalization, profile and W3C
cases pass; strict affected consumers and complete helper hygiene pass. The same
173 constructor sources and original sinks remain, with 40 aggregate proof pins
rebound to source:a0a182593706ad6a. Normal signed commit and push hooks pass.

Four Miri commands select the intended SmallVec library cases. The default,
Tree Borrows, strict-provenance and i686 models retain all 18 storage cases each;
the existing BLAKE3 probe remains. Complete local make miri passes all 73
interpreter case executions. The native semantic chain has no WASM copy.

Current-head independent review reports no actionable finding and no architecture
concern. Its exact-charge finding is addressed. All 50 current-head hosted checks succeed, with five conditional skips
and no review debt or unresolved thread. No CI waiver applies.

Parent #489 remains open for original memory admission, configurable search and
dataset-state controls, heap UDF execution and explicit Datalog governors. This
unit makes no heap-byte containment or fallible-allocation guarantee.

Conflicts: none.

Defect-Class: native-stack-exhaustion
