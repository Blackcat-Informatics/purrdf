Task 1 implemented streaming SHAKE128 and SHAKE256 in the existing native
Keccak home. The public API consumes the absorber into a distinct incremental
output reader and uses caller-owned output buffers. Existing SHA3 modes use
the same absorption and padding implementation.

Independent task review returned PASS against the approved plan and exact
source identities. It independently replayed all four complete NIST answers
from the captured primary PDFs and all 16 OpenSSL boundary/long-output answers.

Validation passed: 129 native affected-package tests, 29 doctests, all-target
clippy with warnings denied, wasm library build, helper census, shard coverage,
formatting and whitespace checks. The unchanged four SHA3 suites replayed
56,388 frozen records. The required staged-index commit hooks ran successfully.

Signed commit: `74bf968ce28da00f9ab6ad054b150c4f5d91da18`
(`feat(hash): add streaming SHAKE128 and SHAKE256`).
Push succeeded and remote branch readback matches that exact commit.

This checkpoint establishes the hash substrate. The remaining approved tasks
are still required for ML-DSA, composite Sign1, writer/resolver integration,
compaction/certification and final qualification. No PR or merge is claimed.
