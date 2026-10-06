# Assembly execution-path report versions

The original read-only recommendation is preserved byte-for-byte at
raw/S2-simd-execution-path-original.md. Its SHA-256 is
995d912d0bd34ea6bbeac52a3fd36af0137d89818ae6fc5b234078e5da3e8958,
matching the historical raw/S2-simd-execution-path.sha256 receipt. That receipt's
original path names the earlier version; it is not a checksum of the amended
current report.

The current reviews/S2-simd-execution-path.md adds two explicit corrections:
local selected Stage evidence is untracked/unignored, so source replay must be
clean; and workflow parity requires the existing make simd-asm entry point.
raw/G3-execution-path-current.sha256 binds the amended report. The original was
reconstructed by removing only those added sections and its exact historical
digest was verified. No measured evidence or old refusal was rewritten as a pass.
