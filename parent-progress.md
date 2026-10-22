Native cause completion is now owned by #677 in the exact stagectl checkout
`.worktrees/677-preserve-native-content-and`. Current implementation removes the
three verified columnar erasures: global integrity verification, blob-row
insertion and complete reconstruction diagnostics. Content digests delegate to
the existing root digest record; no second renderer or hashing implementation.

Actual core insertion/verification tests pass (seven selected). All 31 columnar
library tests, the frozen-byte case and six public structured-error cases pass,
including the existing 18 source-session scenarios. Public corrupt-blob and
invalid-RDF reads retain original native causes after input disposal. Storage
diagnostics retain their detail, location and owned native attachment. Transport
and recursive borrowed inspection record zero allocations, requested bytes,
retained bytes and peak working bytes. Portable execution and strict remaining
qualification are in progress; no PR or complete-parent claim yet.

The initial concern about missing diagnostic Clone was disproved by the existing
resident_clone! implementation before coding. Existing Clone/Eq traits remain.
All four #558 donors are preserved. Parent #558 remains open for its complete
shipping inventory, remaining malformed/UTF-8 producers and host acceptance.
