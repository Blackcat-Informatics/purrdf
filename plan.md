# Compact source directory

Behavior: store the original sorted digest and physical address in 37 bytes per
row. Persist one 32-byte cumulative sum per complete group of 32 rows; populations
are implied by row positions. Reconstruct at most 31 tail members through the
authenticated admitted reader and report actual work. Independently certify
records and checkpoints against RDF rows. The explicit new image profile
refuses older images with an actionable regeneration requirement.

Dependencies: existing `SourceDirectory` checkpoints, `Multiset256`, authenticated
fixed-record streams and original native storage/work accounting. No external
package or upstream implementation.

Exclusions: oversized dictionary records, streaming construction/certification,
common-order scans and coordinate APIs. All parent requirements remain open.

Acceptance: existing source difference and segmented native cases, focused
prefix/checkpoint boundary and rehashed-corruption cases, deterministic size
comparison, affected strict compilation, normal hooks and required hosted gates.
WASM runtime semantics are not selected; portability remains the release gate.

Risk: authenticated image profile, source address/digest integrity and bounded
read work. Preserve failure/cancellation semantics, original row identity and
independent certification. No compatibility decoder or trusted-output shortcut.

Active delivery begins 2026-10-24 20:17 UTC. Reassess after two active hours;
maximum four active engineering/local-validation hours, hosted waits excluded.
One owned Cargo at a time; preserve sibling processes and worktrees.
