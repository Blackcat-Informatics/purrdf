# Manual capacity audit follow-through

This is a manual source review, not the enforced structural census required by
#490. The tool freeze remains in force; no checker, runner, script or census
registration was changed. `recovery-capacity-source-audit.txt` retains the broad
text-search candidates. It includes tests, documentation and invariant checks;
its line count is not a defect count or a complete structural classification.

The current per-site producer/public-boundary evidence remains in validation's
capacity site map. This additional review found a real composed-table defect in
`ir/paged/graph_index.rs::build_csr`: each per-page graph list is bounded locally,
but their concatenated postings are not u32-bounded. All four `GraphPageIndex`
derivation consumers call this producer. Its `u32::try_from(postings.len()).expect`
could therefore panic on an otherwise representable native table. Private CSR
offsets now use the resident `usize` address directly, preserving the actual
postings order and u64 PageId values. Native64 offset entries grow from four to
eight bytes; wasm32 and persisted formats are unchanged. The new direct producer
control and all three original graph-index controls pass4/4 in
`recovery-diagnostic-owner-csr-runtime.log` (terminal0).

The reviewed candidate dispositions below are deliberately narrower than a
claim that every remaining production assertion is harmless:

- `GlobalTermId::from_index` calls the original typed slot check before its
  `index + 1` and NonZero expectation. That expectation enforces the checked
  representation, not a replacement refusal at the capacity boundary.
- `TermInterner`'s `TermDictionary::with_interned` populates each of the four
  optional IDs before the final array conversion. Each fresh term still uses
  fallible `intern`; on failure the suffix rolls back before publication. Its
  expectation asserts the completed fixed-array initialization.
- `TermInterner::term_count` converts an actual resident Vec length to u64.
  The supported native64/wasm32 usize widths do not exceed u64. This differs
  from narrowing the aggregate CSR postings to u32.
- The generic resident `SmallVec::grow` still has capacity/allocation expects;
  its governed reservation counterpart returns typed StorageError and refunds
  the fresh charge on allocator failure. These are physical container APIs,
  rather than the listed logical table-ID minting sites. This observation does
  not silently exempt them from the absent structural census or claim that an
  arbitrary resident allocation cannot fail.

Remaining acceptance: exact final-source runtime/strict results, the enforced
non-test table-capacity census, mandatory workspace/host gates, and full pair
review/integration. None is discharged by this manual search.
