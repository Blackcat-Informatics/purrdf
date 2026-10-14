# Request identity residency design review

Verdict: REVISE the proposed ownership placement before implementation. Sharing
the immutable source identity is sound; charging a bare Arc<SegmentedSnapshot>
only to the existing session descriptor reservation is not lifetime-correct.
No production edit or build was made by this review.

## Actual source and failure

SegmentedRequest currently embeds the full32-byte SegmentedSnapshot alongside
position, bytes and completed. SegmentedOperation::new pre-admits the exact
request vector capacity using size_of::<SegmentedRequest>(); it does not truncate
the caller's request allowance. The unchanged segmented_trig_roundtrip fixture
requests50000 entries under2000000 live bytes. The reported native48-byte
record makes that vector alone2400000 bytes, so refusal at session opening is
explained by the actual layout, not excessive query work. Preserve that fixture,
the request count, exact inspection slice and all evidence semantics.

Requests are appended before provider invocation, completed is updated only on
success, and the digest hashes full snapshot bytes, position, length and outcome
in original order. Evidence retains State and a fixed prefix/digest; equality
compares request values across distinct State owners. None of these paths needs
one identity allocation per request. The inspected in-tree users borrow request
slices and call getters; no caller found relies on copying a request value.

## Required ownership correction

The existing `_descriptor` reservation belongs to SegmentedSession and releases
on session drop. Evidence can outlive every session and retain State.requests.
Moreover, public with_requests can return `requests[0].clone()`, which may
outlive both State and evidence. A bare shared snapshot Arc would then survive
after its descriptor charge was refunded. This is a concrete undercharge, even
when no additional allocation occurs during cloning.

Use one private shared identity owner at the existing session implementation
home: original SegmentedSnapshot plus its original SegmentedReservation. Session
and each request hold an Arc to that owner. Admit its complete layout and Arc
control overhead before allocation, separately from the session descriptor
charge, using the existing reserve_workspace path. That path already carries
the original base reservation and only a Weak State link. It keeps the Budget
allocation charged after an escaped request outlives the operation and avoids
a State -> request -> identity -> State cycle. The identity payload itself
contains no fallible drop work; its reservation is released at the last owner.
Do not retain a provider, session or whole lexical dictionary to solve this.

The shared handle makes a request pointer-sized at its identity field; measure
the actual final record layout rather than treating24 bytes as a portable
guarantee. One allocation per admitted attachment and Arc::clone per appended
request require no new per-request heap allocation. If attachment subsequently
fails after a recorded read, that retained request must continue owning its
identity charge; a failure before recording can release the unused owner.
Retaining the existing base grant may conservatively retain its original fixed
charge after State drops, as other escaped owners already do. Do not describe
that as a new minimal-physical-byte accounting guarantee.

## Public compatibility and evidence invariants

Keep public SegmentedSnapshot unchanged and returned by value. Request getters
should borrow &self so ordinary borrowed-slice use remains unchanged. Dropping
Copy is a source API change, albeit in this unpublished migration; explicit
Clone preserves requested owned inspection without materializing a whole
ledger. The snapshot getter may also lose const because dereferencing Arc is
not a const operation; do not promise complete source compatibility or add
unsafe/packed storage to preserve that incidental qualifier. Numeric getters
can retain const with &self.

Equality must compare snapshot values and original request fields, never Arc
pointer identity or reservation identity. Preserve the existing Debug field
names/value presentation instead of exposing private reservation internals.
The hash preimage remains the full original snapshot bytes and unchanged
numeric/outcome fields, with the existing domain and order. Exact prefix,
with_requests slice, provider-refusal entries, first-fault and final descriptor
checks remain untouched. No request-ID materialization or alternate ledger API
is needed.

## Focused qualification required

- Run the unchanged50000-request/2MB TriG roundtrip fixture successfully without
  increasing limits or changing its workload, and record actual record/owner
  sizes and admitted totals.
- Exercise existing multi-source request identity and immutable-prefix controls;
  equal values from independently attached equal identities must compare equal,
  while overlapping addresses from distinct snapshots remain distinct.
- Clone a request through the public inspector, drop session/operation/evidence,
  then read its exact snapshot/address/outcome and verify the identity and base
  charge remain live until final clone drop. Verify final refund and no cycle.
- Check refusal before identity allocation and after a retained failed read,
  including charge conservation and original first-cause preservation.
- Use the existing allocation probe to confirm repeated request append/clone
  has no new identity allocation per request after the admitted descriptor owner.

No whole-suite rerun is requested by this design review. The bounded source
implementation and actual affected checks must qualify before a PASS verdict.

## Shared native owner refinement

Reviewed the existing purrdf_lex::allocation::shared::Shared implementation after
root proposed reuse. Revised design verdict: PASS for a
Shared<RequestIdentity { snapshot, _reservation }> design subject to the
ownership/API and runtime conditions above. This replaces the Arc suggestion,
not a second shared-ownership implementation. The original bare-Arc/session-only
charge proposal remains rejected.

Shared is one pointer for sized T; allocation_layout includes the concrete
Header<T> count, payload, padding and alignment. Reserve exactly that layout's
size through the existing reserve_workspace home before try_new. Its allocation
failure drops the captured original payload/reservation; map that failure to the
existing AllocationFailed("request identity") and preserve the normal first-fault
latch. Successful ownership keeps the original base pin and Weak State through
the last request/session clone. Shared::drop explicitly moves T out, deallocates
the header, then drops T: the reservation therefore survives physical release
of the allocation it admits. No custom unsafe code or reference protocol is
needed in segmented storage.

Shared::try_clone performs checked count growth without allocation. Its ordinary
Clone implementation calls expect on count overflow, so do not derive Clone on
SegmentedRequest or use the infallible clone internally. Expose explicit
SegmentedRequest::try_clone with its original SharedCloneError result, retain
borrowed getters and value-based Eq/Debug, and document loss of Copy/Clone and
const snapshot lookup on this unpublished API. All in-tree ordinary borrowed
inspectors can remain unchanged; an owned request caller must handle try_clone.

For internal ledger insertion, acquire the shared identity reference before
changing the request vector, I/O total, digest or provider state. A count refusal
must issue no read and leave the exact prefix unchanged; normal query failure
routing must latch that original cause. SharedCloneError is not allocator
failure, a full request vector, or a byte-address overflow. Preserve it as a
distinct small typed SegmentedError cause at the existing error home (with
Display/Error::source), rather than mislabeling it as AllocationFailed,
EvidenceExhausted or AddressExhausted. Public out-of-operation try_clone refusal
need not mutate the completed operation's evidence.

The existing Shared null-allocation and checked-count tests qualify its generic
protocol; they do not by themselves prove session grant/prefix wiring. Retain
the escaped-request and unchanged fixture controls above, and directly review
the segmented error mapping/failure ordering in the implementation. Reusing
this original shared owner provides fallible construction/retention without a
new runtime dependency, vtable owner, duplicated helper or per-request allocation.
