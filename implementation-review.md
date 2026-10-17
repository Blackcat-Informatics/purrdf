# Implementation self-review

This is the implementing agent's review, not an independent review.

- The Datalog trait/helper were removed and replaced with direct core re-exports. Existing consumers retain one identical interface and no adapter or dependency was added.
- The controlled canonicalizer attaches the caller before source collection, shares the existing RDFC algorithm, preserves deterministic least reserved-vocabulary selection, uses the original fixed decrement sites, and propagates typed stop through component, nested-term, first-/n-degree, permutation and serialization work. Partial scratch is private and no identity is returned after an observed stop.
- Complete canonical bytes and labels match both standard hashes on native/lowered RDF 1.2 fixtures; all existing selected canonical refusal tests pass. Scalar sorting, lexical rendering and individual hash updates remain atomic between cooperative polling boundaries; this is not instruction-level preemption.
- Recorded and existing non-recorded preparation share the same constructor body. Recording identity uses one helper; digest streaming polls while borrowing existing canonical bytes. Stop maps to EntailError::Stopped, other canonical refusals retain their diagnostics, and the same Arc reaches parser/preparation/search.
- The initial ontology construction now polls class/data indexing and final encoding. Ordinary question finalization preserves existing behavior through an infallible callback; this avoids replacing an existing honest unknown search certificate with a question-preparation exception.
- The new integration fixture covers actual public proof replay, consistent and inconsistent decisions, post-identity cancellation, digest cancellation, every observed constructor poll, selected schema ceilings and stopped-search receipts. The same seven cases execute under Node/WASM and are registered in the Make lane.
- Public core functionality is re-exported through the existing RDF facade. No source dialect, canonical profile, poison budget, runtime dependency or optimized profile was changed.

Open before publication: final strict Clippy/format and normal hooks. Hosted full CI and independent review have not been established.
