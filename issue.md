# Issue #708: Compact authenticated source directory with sparse prefix checkpoints

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

The effective source directory persists an 88-byte record for every normalized
source row: a 32-byte digest, four-byte physical ordinal, stream tag and padding,
plus a 48-byte inclusive multiset prefix. The prefix population is implied by
position and the cumulative sum can use sparse checkpoints, as the resident
directory already does.

Deliver a complete compact authenticated directory representation. Keep source
digests, original physical row addresses, exact range comparison and native
typed refusal behavior. Store sparse cumulative sums and reconstruct bounded
prefix tails through admitted readers, counting actual work. Remove redundant
per-row population and padding. Certification must independently rebuild and
compare all records and checkpoints against RDF rows, including reifiers and
annotations. Tampering with a recomputed block hash must still be refused.

Use a new explicit image profile and refuse older profiles clearly; v4 does not
require a parallel compatibility decoder. Preserve deterministic bytes within
the new profile and document image regeneration. No external dependencies or
upstream implementation are authorized.

Acceptance: native prefix/row-boundary and source-difference behavior, corruption
refusal for addresses/digests/checkpoints, exact work/admission behavior, and an
image-size comparison on the same deterministic native fixture. Compile actual
consumers and run normal hooks and required hosted gates. General semantics stay
native; do not duplicate them on WASM.

This is the directory-compaction delivery under #705. Oversized out-of-line
dictionary terms remain a separate complete delivery in #705. Neither delivery
completes #564's bounded streaming builder/certification and common-order scan
contract. Keep both parents open until their full requirements are met.

Budget: at most four active engineering/local-validation hours, reassess scope at
two hours; hosted CI/review waits are excluded.


## Comments (0)

