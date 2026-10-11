# Original default row footprint repair

Status: source written; qualification pending. No default-performance PASS.

Root's paired original36 measurements retained five significant regressions,
including DISTINCT53.24%, OPTIONAL11.18%, UNION10.28%, ORDERLIMIT7.95% and
int-versus-double sorting2.60%. Unchanged numeric controls moved in the other
direction; this does not waive default acceptance or attribute all shifts to
noise. The original wide RetainedRow stored the complete unused RowEvidence on
every ordinary row/key/permutation move.

The actual carrier in solution.rs now retains only its original RowStorage
field. Ordinary Resident and admitted Shared cell variants preserve the original
layout and allocation path; virtual evidence attachment is an inline no-op.
Explicit evidence uses an admitted Tracked variant. Its original cells have
one flat shared capsule, independent of the immutable metadata owner. Changes
share that capsule rather than nesting prior metadata controls; exact unchanged
metadata reuses the original owner. Both shared controls are admitted before
moving ordinary cells, and bounded moved resident spills retain a grant for
their actual capacity. Native refusal leaves mutable combination's original
row/cells/grants intact. Worker transport extracts only unique resident cells
and their metadata; admitted/shared cell owners cannot be detached.

Physical virtual-empty checks include id, owner, membership/closure flags and
original rank. Exact unchanged-attachment equality additionally compares the
original owner identity and rank. The public semantic is_default law and the
coalescing same_query_record default law remain unchanged. No default-supported
rank or explicit owner is elided under those semantic shortcuts.

Every actual RetainedRow attachment now returns Result and receives the original
caller workspace: BGP, path, property-function publication, shape publication,
projection/graph/group/aggregate, expression/VM results, OPTIONAL/UNION/join,
SERVICE and worker reinterning. Original producer envelopes retain their own
already-admitted infallible metadata setter; this does not allocate the new
RetainedRow carrier. Original native errors are propagated, never stringified or
converted into unknown evidence.

The existing shared producer-ingress case additionally runs default-supported
rank1 and rank2 through actual property production, BIND/FILTER/DISTINCT/ORDER,
clone and accepted-row iteration without recomputing ranks. The original unit
transport case covers explicit dictionary ownership and rank1-to-rank2
attachment, shared original cells and borrowed accepted row identity.

Consolidated strict1 session21203 failed with six caller-context/type errors;
all were collected before one correction batch. Strict2 session7236 completed
all three packages and failed only two evaluator assertion pointer lints.
Those were corrected together; final evaluator all-target strict93677 is
terminal0 PASS, reusing unchanged text/retrieval strict2 source coverage.
Whole original evaluator/text/retrieval native family51149 and existing complete
make wasm-test portable71464 are running against frozen Rust source. Native,
actual portable, original50-allocation, physical scaling and current paired
performance are NOT yet qualified for this changed shipping source. Earlier
controls qualify their compiled source only. Final unchanged make check remains
pending after all required source corrections and acceptance lanes settle.
