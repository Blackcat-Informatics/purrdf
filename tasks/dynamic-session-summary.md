# Demand-grown request and cache admission

Published checkpoint `b5db98eb7` removes the request/cache maximum arrays from
session opening. Their buffers now grow through the existing QueryStorage and
Memory admission home. The original typed causes survive. Request limits are
checked against counts rather than incidental vector capacity. Cache capacity
growth is admitted before block birth, and its grant has no State backlink,
avoiding recursive lock acquisition. State fields destroy both buffers before
the original storage reservation is dropped.

All 14 existing segmented-stack controls pass. The extended 1000-source header
control also opens with maximum u32 request/cache ceilings under 256 MiB, with
under 16 MiB cumulative allocator traffic and peak, and zero retained bytes.
That registered control passes on actual WASM. The four native session owner
controls pass: escaped request identity, shared request/cache/source admission,
first source exhaustion and workspace lifetime after session destruction.
Warning-denied all-target Clippy passes. Normal commit/push hooks passed.

These receipts establish physical demand growth and preserved ownership; they
do not establish the full 200,000-claim consumer or explain its image layout.
Those remain active tasks. This is owner source assessment, not independent
review or whole-issue completion. No runner or host tooling changed.
