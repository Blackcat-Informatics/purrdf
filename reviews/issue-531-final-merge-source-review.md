<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Final application merge source review

Scoped source verdict PASS; compilation, semantics and stack-byte acceptance
NOT MET. This is the actual source-only binop.rs delta after correcting the
right-nested failing control, not approval of the earlier left-only candidate.

Only the final schema union, output ceiling, compatibility/projection row merge
and Lift finish move to finish_application_output after per-row collection.
Lift, shared left schema, right schema and original admitted pair buffer move
without cloning. Endpoint pre-admission, property-function interception, recursive
right evaluation, per-row truncation absorption/discard and declared-row cap
remain in their original order. No new heap owner, grant or semantic policy is
introduced, and existing error propagation remains fallible.

Two initial review findings were corrected: the helper explicitly stays out of
line, consistent with existing finish_left_join, and its generic ApplicationMode
retains original compile-time M::DECLARED specialization instead of a new runtime
bool. Neither correction changes tooling or compiler configuration.

The original left bag still lives in the caller over the helper call; this is
not a claim that all retained stack state or admission owners were eliminated.
The intended benefit is only removing final-merge temporaries from the frame
held across recursive right children. The optimizer and actual WASM execution
must establish that benefit. Original retained-artifact slope is4080 bytes per
added LATERAL level; neither unchanged depth floors nor any EXISTS requirement
has passed on this new source. Keep fixture and stack-correction commit boundaries
separate and attribute checks to the actual source/artifact they execute.

Current lint-corrected source retains schema and pair owners in the caller and
borrows them in the helper; only Lift moves. The immutable context reference is
sufficient for the unchanged finish path. ApplicationPairs names the existing
admitted pair type rather than adding an owner. Affected retrieval, Geo and
evaluator strict checks pass; focused runtime and actual WASM stack/floor
qualification remain pending. This supersedes the owner-movement description
above for the current source.
