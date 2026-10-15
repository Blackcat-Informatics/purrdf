# Actual codegen artifact selection

Published correction `78b07bb6d` excludes dependent library metadata from the
bench cargo-check graph. Real library/rlib codegen graphs and the primary bench
artifact remain included, with their original per-graph membership. No vector,
FMA, relaxed-instruction or mnemonic criterion changes. The legacy script's
non-Rust line count does not grow; self-tests and normal signing/hooks pass.

Current native diagnostics originally reported frozen lookup as zero, including
an ARM AES-floor refusal. The retained full-codegen core assembly contains the
actual public lookup with 228 instructions and 13 vector operations, including
four aese/aesmc pairs. Replaying the existing parser, selector, counter and
evaluate function over that captured body satisfies the unchanged AES criterion.
The bench check separately reports dependent core rmeta output, not a runtime
library; it must not be counted as a second shipping function copy.

The original complete producer is terminal/cancelled. Its partial state earns no
complete projection credit. Ordinary CI and new complete projection run
38640058325 use the published correction; projection job 116112989085 is confirmed
in progress. Generated document reconciliation and final hosted review/checks
remain open. This is owner diagnosis, not an independent approval or merge gate.
