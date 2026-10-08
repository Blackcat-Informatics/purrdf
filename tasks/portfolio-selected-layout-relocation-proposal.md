# Exact selected-layout relocation proposal

2026-10-08. **READY FOR ROOT ADMISSION; no relocation/removal/selected archive layout change or capture executed.** Qualified private package remains `/opt/purrdf-454-evidence-package.GD9kNHfZ`. The exact operation inputs are under its relocation-proposal directory. Preparation does not freeze current hosted/report writers.

## Exact operation manifests

`removal-set.tsv` has265 rows, each with original absolute path; future private preservation destination; original mode/size/SHA; future selected canonical object path; stored SHA; codec; Stage-relative original path. These are exclusively mapped historical raw records larger than1MiB, totaling5,480,873,749bytes. No task report, plan, validation, squash draft or current hosted report is in the removal set. All nine currently oversized publication files are covered.

Original byte/mode preimages and complete reconstruction were requalified against current originals: actual78080 terminal0/all265 records. Each current size/mode/SHA matches the accepted private map; each stored SHA matches; each gzip passes integrity, complete decoded cmp and original SHA; raw/original-gzip objects pass full cmp. Readbacks are in current-preimage-reconstruction-readbacks.tsv. **No original has yet been preserved by relocation or removed.** Future physical preservation root is `/opt/purrdf-454-evidence-package.GD9kNHfZ/preserved-stage-originals/`, mirroring every Stage-relative path, before any removal. The root/destinations are currently absent and must be exclusive, not adopted over existing work.

`copy-set.tsv` has4,929 exact source→selected-destination rows, each with mode/size/SHA. It places all4,882 qualified canonical objects plus47 reconstruction/inventory/readback files beneath `.stage/rdflib-shim-algebra-level-reassignment/raw/final-lossless-evidence-archive/`. Actual94630 terminal0 read every copy-source hash. Existing source tar and private rejected-prose history are absent. The uncompressed rendered data tar is absent; its qualified compressed object and164-member metadata remain. No live member JavaScript files are added.

`retain-readable-paths.txt` records all other currently selected file paths. Primary small/current reports, acceptance plan, validation, squash draft, hosted job/attempt metadata, conformance projection records and all unrelated raw small records stay at their current readable paths. `current-large-unmapped-holds.txt` is explicitly retained and currently empty: any new/stale large file absent the qualified map is a **HOLD**, never automatically removed. This exact set is not a dynamic remove-everything-over-threshold rule.

## Admitted-operation sequence proposed

1. Recheck every selected source/destination boundary, current265 original preimages and all copy-source hashes. If any mapped original changed, any destination already exists, any symlink/special file appears or any size/mode/hash mismatches, stop and report; do not broaden the removal set.
2. Create only the exclusive private preservation root, copy each exact265 original with mode/timestamps into its listed mirrored destination, verify full cmp/current SHA/size/mode and reconstruction, then record all successful preservation readbacks. All265 must be safely preserved before any original removal. No source, sibling, source tar or unrelated artifact is moved.
3. Create only the exclusive selected raw archive destination and copy the4,929 exact qualified sources with original modes. Verify every stored copy SHA/size/mode, every canonical object and complete original-path decoding/cmp. Copy the admitted relocation manifests/readbacks into the selected archive as documentation; do not restate matching credential literals.
4. Only after all preservation and selected-copy validations pass, remove **only the265 exact listed original paths**, rechecking each source's unchanged current preimage immediately before removal. Never recursively remove directories or use a threshold/glob. Preserve every full original outside selected publication until remote archive verification and root-authorized cleanup. Any interruption leaves private originals and already verified copies intact for diagnosis, not a reason for force deletion.
5. Reconcile every original role/path with its canonical map and retained readable files, measure actual selected size/largest blob/unique contents and rerun raw/decompressed policy and reconstruction checks. Root admission for these operations does not itself admit capture, publication or cleanup.

The map restores original mode/size/SHA and exact names. Newly compressed codec gzip decodes; raw and raw-gzip copy stored bytes unchanged, preserving the originally compressed record exactly. The265 preservation originals make replay independent of compression or a removed original path. Baseline/head source bytes reconstruct through canonical Git/audit ancestry and verbatim path/hash manifests; no full-source copy, source redaction, policy weakening or opaque compression concealment is introduced.

## Measured selected-size prediction

At exact94630 inventory: current selected file bytes5,807,710,176 minus listed originals5,480,873,749 plus qualified archive copy bytes1,492,337,014 = **1,819,173,441 selected file bytes**, before this proposal, operation readbacks and ongoing small report growth. The largest qualified object remains87,570,494bytes. The unique stored object set is1,480,453,834bytes; retained readable duplicates and canonical object copies share content but their actual Git blob/pack behavior must be measured rather than assumed. Predicted file-byte fit is below2GiB; **actual pack/single-push acceptance remains unrun**.

Live writer exclusions: no deletion or freezing of tasks/current-hosted-6ce3, current hosted qualification index, validation, squash/current PR artifacts or other small reports. Current hosted jobs and their owner continue normally; old ed14 failures remain historical. If any live output grows beyond1MiB after this manifest, it remains held/readable until separately classified. Final stop/freeze/current report-version/map refresh is deferred to the separately admitted capture phase, not required to relocate these static historical raw originals now.

## Supported capture-preview route, prepared only

Actual ghprsq defaults GHPRSQ_GIT to `/usr/bin/git` and resolves the existing helper to `/home/paudley/stage/stage-scripts/ghprsq-stage.py`. Its supported operation is `capture GIT STAGE TEMPORARY`; main dispatch and normal ghprsq call it directly. After final writers stop and source/check/review prerequisites are satisfied, root may separately admit:

```sh
preview=$(mktemp -d /opt/purrdf-454-stage-capture-preview.XXXXXXXX)
python3 /home/paudley/stage/stage-scripts/ghprsq-stage.py capture /usr/bin/git /home/paudley/Active/purrdf/.worktrees/454-rdflib-shim-algebra-level-reassignment/.stage/rdflib-shim-algebra-level-reassignment "$preview"
```

This is the existing first-party helper, not a new private checker or novel ghprsq flag. It scans stable snapshots, hashes all selected bytes with no filters, uses only a temporary index, writes/validates a tree and returns tree/filecount/bytes. It creates local Git objects but does not sign a commit, update refs, push or merge; none was executed here. Its inventory rejects changed writers/special files and its credential policy remains unchanged. Temporary task-owned preview files may be removed after inspection without touching published evidence. Exact final push-pack feasibility and prerequisite freshness remain separate root-owned checks; successful preview alone does not satisfy the2GiB forge limit or authorize final ghprsq integration.
