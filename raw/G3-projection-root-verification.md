# Hosted projection artifact and clean-source verification

Run37532305791, job112504470023 completed SUCCESS. Generation, all seven
configuration qualification, aggregate, unchanged compiler and upload steps
passed. Native artifact11445534280 identifies head8364b29c9fb70195f123e2bc084160d83f78850a.
Its ZIP digest cda92e37b3cb12f92735f2e3ab8880ffa5db36193d2e039ee2c9867e87923e89
matches the downloaded bytes. Archive inventory has no absolute/traversing paths
or symlinks. All bytes, primary native metadata/job logs and extracted evidence
remain under raw/G3-projection*.

All16 internal projection-file checksums passed. Their native target/ prefix
was removed solely in the separate relative checksum receipt to match archive
extraction paths; original hashes and receipt are retained unchanged. Compiler
before/after bytes match, and match the independently analyzed normal-run compiler
EA137335/LLVM23.1.3. Report status passed, schema1, exact seven configurations,
110sites each. Aggregate says seven matching successful reports. Generation and
qualification logs both say complete gate; qualification reused verified build
artifacts, as its actual timing receipts explicitly record.

Clean replay qual-asm-37532305791 began at head8364b29c/treecb77b4d6 with empty
source status. All13 hosted before-input checksums passed. The sole downloaded
native projection.patch applied cleanly; resulting document matches uploaded
bytes, all other tracked paths remain unchanged, status equals hosted status
with only docs/design/purrdf-simd.md modified. Both after-input checksums pass.
Existing runtime.source_identity returned
fe73fce1722108d47b7fcb7a8bcbf6214318a23800304c759282753c41b005c0,
exactly the successful hosted report identity. No local compiler/Cargo/driver
measurement or toolchain selection was invoked.

Projection patch SHAbeeac41c6acb5d419762f566f05988cef7a746bdea8c66ce384e0af2723f3772;
document SHA7285a4c4d1f401aec02d880622cdf7f9893fee4b9d781d27a61c076cd6a94356;
manifest SHAc96ab1574d88cae967d67bcb642a7a4b7716555aaf9ca66e1b2e87fac36f760a.
The exact generator changes only the core.paged-map-quad v3/v4 vector counts
14/10 to0. Existing rules, selectors and all other cells remain unchanged.

One additional verification command mistyped a second status-file path and
returned2 after its document/other-path/after-input checks had passed. It made
no mutation. The corrected fail-fast command compared the actual hosted and
replayed status files, document bytes, other tracked paths and both after-input
checksums, all exit0. No typo failure is presented as a passed command.
Root's first JSON compiler-string extraction used jq -r, which adds a newline
after the string's own trailing newline; that representation compare returned1.
The corrected jq -j extraction preserves the actual 214-byte string exactly and
its receipt compare exits0. This is an extraction-format correction; actual
hosted before/after compiler bytes were already identical.

This qualifies artifact/source correspondence and actual hosted projection;
G3b implementation review/normal signed transport and final ordinary independent
seven-shard CI/aggregate are still required. Clean replay must be reversed to its
owned clean state and removed without force after final evidence review.
