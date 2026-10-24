Columnar reads currently allocate expected metadata maps and encoding lists on
successful validation, then copy every decoded blob payload into its content
store. Borrow those expected values and move the checked original payload instead.
Metadata insertion also moves keys, and the five-table writer fills its array
directly.

All 40 native Columnar cases pass. Validation allocates nothing for all five
tables and both codecs; 1 KiB and 1 MiB payloads retain their original pointers
with identical transfer traffic (1 allocation / 244 requested bytes). All ten
frozen file hashes remain unchanged. Strict all-target Columnar Clippy passes.
Normal commit/push hooks and current-head hosted checks pass, including the
complete release WASM build. Hosted CI reports 49 successful checks/statuses and
four manual-only skips, with no failures:
https://github.com/Blackcat-Informatics/purrdf/actions/runs/40113480023.

The additional local workspace replay was stopped under the updated validation
policy (exit 143); no pass is claimed for that interrupted run. Affected native
and strict checks and exact-head hosted qualification completed independently.

This extracts only the committed allocation behavior. The unfinished grammar
migration and later page-writer changes remain preserved in their donor checkout;
the complete contracts in #688 and #558 remain open.

Closes #702.
