Preserve native Datalog handles during indexed removal

Replace scheduled survivor reconstruction with indexed native row removal. Batch,
tail and append-log cells retain vacant handles without moving surviving rows;
reinsertion receives a fresh append identity. Empty partitions leave live
enumeration while retaining their lookup coordinates for reactivation. Native
wide dictionary IDs and source failures retain the original source boundary.

Distinguish live cardinality from retained append extent throughout Datalog and
Shapes. Deltas and row-indexed buffers use retained slots, and seminaive/chase
governors charge that storage. Compaction removes retired keys without restoring
old handles. Clarify sparse count and reinsertion behavior in internal comments.

Qualification: 435 native library cases, 48 integration cases and one doctest; 32 actual public
Node/WASM cases; 36 Shapes rule-engine cases; strict all-target Clippy,
warnings-denied rustdoc, helper hygiene and normal signed commit/push hooks.
Three removals from 4,096 rows plus the warmed object scan allocate zero bytes.
Independent substantive review covered the original implementation and occupied-tail repair. Its minor prose spacing finding is fixed. Current hosted-data projection is mechanical and was skipped as similar by the later review; no fresh source review is claimed.

All seven hosted kernel measurements met original floors and mnemonics.
Incidental catalogue counts differed between local compiler 21b707e3f and hosted
19833af72; exact hosted measurements were projected through the existing writer.
Fresh configured hosted qualification on a7dd78521 remains pending; finalize this draft
only after those checks and refreshed feedback/integration pass.

Vacant append slots and interned terms remain retained and charged. Full signed
transactions, recursive/negative maintenance, current witness repair, governed
updates, affected-part work and maintained refutation remain in parent 493.
No parent completion or bounded long-lived memory is claimed.

Closes #669.
Defect-Class: none
