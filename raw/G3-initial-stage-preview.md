# Preliminary selected-evidence capture

At committed head 8364b29c9fb70195f123e2bc084160d83f78850a, the freshly
inspected existing ghprsq-stage.py check-clean and local-only capture operations
completed successfully. Capture result: tree
e459f04c2680505ed32ee126664a2d0a923010c9, 257 files, 2506732 bytes.
raw/G3-initial-stage-preview.txt preserves the exact result.

The helper scanned stable raw bytes for credentials and verified every captured
path, original mode and blob against its temporary-index tree. It wrote local
Git objects only; no signing, publication ref, push or merge occurred. The owned
temporary index/result files were removed and their temporary directory removed
without force.

This is an early compatibility check while hosted assembly runs, not a frozen
final archive. This receipt itself and every subsequent artifact change invalidate
the preview's exact inventory. Final evidence writers must stop and the existing
capture must run again before actual ghprsq integration. Hosted artifacts and
remaining completion/review gates are still pending.
