# Cancellable recorded DL preparation

Issue 582 supports the required native class boundary in GMEOW 1803. Preserve the unrelated REWORK_LIST.md and text-index worktree/PR. Repository goals, baseline, ring-fence, one-home rule, RDF 1.2 canonical profile, fixed poison budget and existing build profiles apply. No resource probes/caps, local GMEOW corpus producer, clock, new dependency or hook bypass is permitted.

## Task 1: One stop interface and controlled canonicalization

Move the existing latching StopSignal trait and optional-signal poll to core governance; Datalog re-exports that exact implementation, with no second trait or adapter. Add a controlled DatasetView canonicalization entry point and typed CanonError::Stopped. Share the existing canonical algorithm. Poll before source work, while collecting/admitting components and nested terms, at first-/n-degree and permutation work, and while serializing output. Do not publish partial canonical bytes. Preserve complete bytes/labels, hash selection, reserved-vocabulary/read diagnostics and fixed poison-budget charging. Keep uncontrolled wrappers on the same code path.

## Task 2: Recorded DL preparation with the same signal

Add a recording constructor taking an explicit schema preparation budget and caller signal. Compute canonical input identity cooperatively, map typed cancellation to EntailError::Stopped, and retain the same signal in native ontology preparation/search. Reuse one identity implementation from existing recording constructors. Preserve successful proof identity and replay, source diagnostics and unknown/stopped decisions. This does not claim shared-store/delta or rooted proof support is complete.

## Task 3: Verify, publish, integrate

Check immediate and mid-operation cancellation, symmetric blank search, RDF 1.2 statement-layer handling, SHA-256/SHA-384 complete parity, existing canonical refusals and exact poison budget. Exercise public recorded DL construction, native proof replay, preparation and search cancellation. Run affected core/Datalog/entail tests, strict affected Clippy and real WASM checks with existing profiles, then normal hooks. Publish a focused PR, collect CodeRabbit feedback, use ghprsq under existing authorization, archive evidence and clean up only owned work. No independent review is fabricated when unavailable. GMEOW pin/adapter adoption follows the actual native merge.
