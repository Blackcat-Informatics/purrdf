# Diagnostic evidence capture preview

The current live first-party ghprsq helper's local-only `check-clean` and
`capture` operations both exited 0 after all assigned preparation writers had
stopped. The actual result is in G3-diagnostic-stage-preview.txt: tree
d36b604da91f932c21020d1cad20d5aaa7af4c32, 467 files, 56892859 bytes.
This includes the complete extracted failed-shard evidence, uploaded ZIPs and
independent actual-body/completion/review-debt preparations. Raw-byte credential
checks and file/mode/inventory stability passed. No symlinks were present.

The temporary index/log directory was removed without force after copying the
receipts here. Capture wrote local Git objects only; no signing, refs, push or
merge occurred. This is compatibility preparation, not final archive or merge
acceptance. Adding these receipts already changes the inventory; pending hosted
projection and final reports also invalidate this preview. Repeat stable capture
after all final evidence writers stop, and verify actual published archive before
cleanup.
