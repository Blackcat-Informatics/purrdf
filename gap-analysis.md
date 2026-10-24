# Source review of the complete allocation component

This is the implementer's assessment, not independent approval. Actual hosted
review covers the final PR head with no actionable code finding. Reviewed all three changed production files,
the schema/column invariants and actual read/write entry points.

Metadata keys remain BTreeMap-ordered. The two expected pairs are in the same
lexical order as their keys. Iterator equality checks complete cardinality and
values; mismatches still construct the original diagnostic only on refusal.
Entry insertion retains the same first duplicate key and human presentation.
Encoding lists retain their exact PLAIN/RLE sequence through static slices.

TableData validates column shape/row count before decode_blobs. Both original
bytes-column checks remain before splitting, and each row checks digest UTF-8,
hex syntax, lowercase, strict order and required payload before moving it.
ContentDigest parsing admits only 64 ASCII hexadecimal digits, making the
uppercase check equivalent to equality against its lowercase rendering. The
original ContentStore checksum producer remains the final admission point;
its actual original-cause capture passes after input destruction.

The fixed table array preserves Table::ALL order and propagates each actual
write failure. No malformed partial output escapes. Tests prove original blob
pointer/bytes and constant allocation traffic at two sizes, all five table
schemas/two codecs, all ten frozen outputs and existing source/refusal ownership.
All 40 native cases and strict affected all-target Clippy pass. Normal hooks,
exact-head hosted checks, release portability and independent hosted review pass.
The optional local workspace replay is interrupted (exit 143), not a pass; the
replacement AGENTS rule limits manual checks to affected scope. The unfinished
grammar migration is excluded and both parent contracts remain open.
