# Reuse the existing current-support replay

stagectl's issue/comment and prior-attempt coverage is complete for this issue:
zero issue-specific commits or other named branches and one citing PR, #592.
That PR explicitly states it does not repair this failure. Generic defect-class
trailer tallies are not a recurrence count for this exact bug.

The relevant existing implementation is PR #587 / 04f6edbff: one predecessor-linked
support journal and Hyper::replay_support. The producer records raw current
frames and observed body facts; the consumer replays licensed applications and
then compares merge-invariant node references and facts. Keep this single home,
with no alternative proof checker or search-based acceptance fallback.

PR #589 / issue #588 refreshed cross-host v3/v4-to-v5 artifacts. The issue reports
that packaged WASM and Python artifact checks pass while the real imported CLI
producer/checker fails. Replacing a frozen proof or relaxing an assertion would
address a different problem. Local reproduction and inspection must distinguish
an actual missing application from an identity/representation mismatch.

Existing proof tests cover ontology/byte-string replay, derived witnesses,
missing heads/frames/predecessors, branch ownership and source identity. Retain
and select the relevant cases after diagnosing the computation. Add only the
missing imported/multi-individual or other demonstrated control through the
existing native and portable harnesses. No donor port or new dependency is needed.
