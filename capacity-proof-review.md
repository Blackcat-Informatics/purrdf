# Capacity closure proof refresh

Complete helper hygiene initially failed on40stale capacity proofs and their
40undischarged sinks; this is an introduced pin invalidation, not a baseline
failure. Existing helper-census --index was run against changed checkout and
unchanged canonical main6f1199b1d. Exact projected capacity sources, constructor
membership/identity and errors are retained as raw/capacity-{changed,base}.json;
both actual executions exit0, baseline has no source errors, changed has40.

All167core constructor-source memberships match. Only core content_store.rs
changes, from two borrowed StructuredError impls and their owner tests. No new
TermId/FrozenTerms/global index constructor, private field, capacity conversion,
reserve or authority path is added. Original insert/verify/hash code is unchanged.
The gate conservatively pins whole tokenized files containing macros/derives,
including test tokens, so these reporting additions change the closure identity.

Every one of40proofs keeps its original sink, line, class, rationale and authority
files. Exact generated capacity-source identities of every authority match both
baseline and changed checkout and the existing ledger. Each failed proof had
exactly one matching sink; no duplicate, newly detected sink or unrelated source
error. The only required update is the constructor identity, from
source:3eaf77a07a04eb3d to source:dd5ec90296044816. Those40pins are refreshed after
this review. No census code, exemption, class, constraint or source authority
identity changes. Full helper hygiene rerun session42341 exits0, including all
seeded controls, complete source census and domain registry checks.
