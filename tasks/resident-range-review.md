# Resident dictionary range correction

Owner design/source review; execution status remains in validation.md.

The complete-query refusal is caused predominantly by forward dictionary probes,
not reverse lookup. Two remembered ranges fail when three retained payload pages
are reused, even with the pages pinned during warming. Increasing a fixed hint
count would preserve that defect for larger working sets.

The replacement indexes only resident dictionary pages. A certified page holds
at most MAX_RECORDS_PER_BLOCK dense insertion IDs. Dividing absolute IDs by that
same maximum means one page intersects at most two buckets. Bucket lookup is
hashed by the certified source and bucket; the range predicate selects its page.
Exact resident hits alone still failed the full COUNT on the initial revision.
Cold lookup now also examines the two adjacent buckets and narrows its ordinal
interval from their authenticated bounds. Nonempty dense pages imply at most one
page per intervening ID; the original record cap bounds the opposite end. Thus
an immediate successor follows the known page in exactly one page probe. This
uses observed page bounds, not a guessed term-size or population model.
The number of same-bucket candidates is bounded by that fixed ID width, not the
number of historical terms or configured cache slots. Ordinary block hits retain
the original indexed recency links. No source-wide directory, ID remap, payload
copy, additional pin or alternate decoder is introduced.

Both native tables use reserve_table_with_memory and the cache's original
QueryStorage. State layout covers their inline controls; CachedBlock layout
covers its range bounds. Original payload authentication, exact request evidence,
first-error latch, clone ownership and last-owner release remain in their homes.
Eviction removes the resident page's one or two range entries before unlinking
its cache owner. The table allocation remains charged until destruction.

The first implementation reserved two index entries before every dictionary
read. Its 32 native integration cases passed, but the existing exact pending-block
admission control found unnecessary table growth. The correction reserves primary
cache/payload capacity first, then obtains the exact one- or two-entry requirement
from authenticated decoded bounds. It admits index growth while the pending payload
is already charged, before publication. A failure latches the original cause and
drops that unpublished payload. This avoids estimating a page's physical shape
or relaxing any memory ceiling. The exact-admission control is running; no pass
is asserted here.

The overlapping-source unit now also exercises IDs above 2^53 and a payload page
crossing a bucket boundary. Both source-qualified entries must resolve to the
same physical page while the other image retains a different page for that ID.
That strengthened unit still requires execution on the final correction. The
final reader's 32 native integration cases pass, including an exact 81 data-read
control (one header and 80 dense payloads) and read-free two/three-page warm reuse.
The public fixture has since moved its high-ID start to cross a bucket boundary;
that additional edge execution is pending. The original full evaluator run on
this final reader is also pending; the initial resident-index failure is retained
in tasks/range-index-evaluator.log and is not relabeled as a pass.
