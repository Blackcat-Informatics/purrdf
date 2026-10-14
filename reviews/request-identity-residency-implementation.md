# Shared request identity implementation review

## Current evidence update — 7a3f96ef7

Scoped implementation and affected validation PASS. Read the published two-file
commit and retained request-identity logs: owner controls4/4, unchanged
50,000-request/2MB TriG1/1, segmented13/13, stack14/14 and actual query22/22 all
pass. Final warning-denied core Clippy and wasm32 core check finish successfully
in request-identity-clippy-final.log and request-identity-wasm.log. No duplicate
build was run by this reviewer.

The sole intervening lint correction uses explicit `&raw const` references in
the new unit's existing shared-payload pointer comparison. It preserves the
comparison and does not change production ownership, admission or allocation.
The previous source analysis below remains applicable; no material new source
finding was identified. The original failed strict receipt remains retained.
This qualifies this residency repair, not the whole issue pair or a later full
gate. The historical pending statement below records the earlier review state.

Verdict: PASS for the inspected session.rs/mod.rs production wiring. Actual
escaped-owner and unchanged TriG runtime controls remain pending root's51422;
no runtime or whole-S1 success is inferred here. Review is source-only.

The implementation resolves the plan's session-only reservation lifetime defect.
Each attachment reserves the exact Shared<RequestIdentity>::allocation_layout
before try_new, through the original reserve_workspace path. The resulting
RequestIdentity owns its reservation, which carries the original base grant and
only a Weak link to State. Session and recorded requests share that identity.
No provider, session, dictionary or strong State is retained by it; there is no
State -> request -> State ownership cycle. Shared's existing final-drop law
deallocates its header before dropping the payload/reservation. An escaped
request therefore keeps the identity and base allocations charged until its
last owner is released. Session descriptor charge remains separate.

On identity allocation refusal, try_new drops its captured original reservation
and the enclosing open operation releases the local descriptor. The fixed
AllocationFailed("request identity") cause is latched without diagnostic text
allocation. Failure after a recorded header/provider read leaves the identity
owned by that exact request in State, as required. There is one admitted identity
allocation per attachment, and no new allocation in request try_clone.

Internal request construction evaluates Shared::try_clone before Vec::push runs;
the I/O counter, provider call, completed bit and digest updates all follow it.
Count overflow therefore cannot issue a read or append a phantom request. Its
SharedOwnership variant preserves the actual SharedCloneError and participates
in the existing Error::source macro; the enclosing block-result handling latches
it through the original first-fault path. Public try_clone returns the original
typed clone error and does not mutate operation evidence. No infallible Shared
Clone/expect was introduced in this path.

Request equality compares the full snapshot value and original position/bytes/
completed fields, not allocation identity. Debug retains the original struct
and field names/order/value formatting without exposing reservations. Digest
construction still consumes the original32 snapshot bytes and unchanged ordered
numeric/outcome fields. Evidence prefix/with_requests APIs remain exact; there
is no truncation, prefix materialization or allowance reduction.

Public request ownership intentionally changes from Copy/Clone to explicit
fallible try_clone on this unpublished S1 surface. Getters borrow &self; the
snapshot getter ceases to be const, while numeric/boolean getters remain const.
SegmentedSnapshot itself is unchanged. These are source compatibility changes,
not a claim of identical trait bounds. The new method documents its fallible,
nonallocating retention behavior; ordinary inspected slice callers still borrow.

The new regression genuinely escapes a cloned request, drops both session and
evidence, checks original base+identity charge remains, then requires zero live
charge and no remaining Budget owner after final drop. It also inspects shared
identity reuse, value equality, snapshot/completed and Debug shape. Its record
size assertion and unchanged50000-request/2MB TriG fixture directly cover the
observed residency failure without raising limits. Existing Shared protocol
tests qualify allocation/count machinery; this new runtime must qualify the
segmented ownership wiring. Existing multi-source/prefix/first-fault controls
remain the appropriate affected regression family. No new shared protocol,
unsafe code, dependency or duplicate helper implementation appears in the diff.
