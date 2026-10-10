# Native kNN invocation and cursor ownership

Concrete source proposal: knn-cursor-invocation-owner-draft.patch. Apply check
passes against the active source; invocation postimage is syntax-formatted only.
No compile, semantic, allocator or complete qualification is claimed.

The preferred integration is now knn-combined-cursor-owner-draft.patch, one
coherent unit incorporating the subsequent lookup and diagnostic work. Its apply
check passes against the current shipping source and its invocation postimage is
syntax-formatted. It additionally requires the shared NativeDiagnostic owner.
Use this combined unit instead of applying the constituent patches again.

One shared opening body now copies count/query/bound terms through the native
fallible term home, retaining grants beside the cursor's private invocation. It
also retains the exact four-cell bound metadata buffer and the grant-vector
buffer (at most six term copies). Resident public invocation opening extracts only
resident payloads. Numeric count validation calls the actual admitted native XSD
parser; integer-family gating preserves rejection of noninteger datatypes and
BigInteger values. Membership count construction admits its actual twenty-byte
u64 formatting destination and datatype before fallible allocation.

One pull body now retains the ranked vector owner, admits the one-element
membership buffer, constructs the canonical distance lexical through inline IEEE
text and a fallible admitted destination, clones the three echoed terms through
their native home, and returns the existing admitted row carrier. Bound-cell
comparison uses the actual admitted terms_equal home. Ceiling decrements only
after all bound cells agree. Resident next extracts resident rows; bounded rows
are emitted through next_admitted and cannot escape as a raw Vec.

The embedding relation's actual open_admitted implementation admits the concrete
cursor Layout before construction and uses the existing fallible sized-box home.
Its immutable source Arc clones allocate no new shared controls. Native producer
and retained invocation/result fields drop before their grants.

Required remaining seams before qualification:

The first three items below describe the initial unit. The combined unit now
includes the fallible total-order binary lookup and workspace-carrying membership
distance dispatch, and migrates Function/Data messages to the shared diagnostic
home. It removes recursive value Debug from the invalid-count diagnostic while
retaining the argument position and noninteger reason. Native rows() invariant
diagnostics still require ownership. Test assertions must preserve exact error
classification when matching the new carrier's kind.

- TermRows::row_of still calls native ordering through its old infallible
  comparison work-list. Use a fallible native try_cmp + capability terms_cmp at
  the same comparison home, preserving binary-search ordering and complexity.
- Error formatting still uses existing dynamic Function/Data/internal strings.
  Integrate the shared NativeDiagnostic owner; nested TermValue Debug formatting
  needs its own admitted/fallible walk, not merely a String lease.
- The membership lookup's concrete distance path needs the workspace capability
  for its diagnostic owner. Its numeric computation uses stack arrays only.
- HNSW uses this shared cursor but still needs actual traversal-buffer admission
  and native admitted invocation opening; an opaque-source refusal cannot qualify
  native index support.
- All allocator-refusal, capacity/overflow, retained clone/extraction/drop,
  cancellation/governor, filtering/ceiling and resident/bounded parity checks are
  required. Existing test API assumptions may need honest carrier migration.

The source writer integrates this coherent unit together with remaining owner
seams and qualifies the settled source in grouped runs.
