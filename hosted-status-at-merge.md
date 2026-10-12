# Hosted status at directed merge

PR #534, head `3c92cbeebad8d52bac064f1d4f9a714e7d9b7fb6`, base `29661e74bdfa5e760fca5595e72b572176d7dd0e`.

The first hosted readback showed failures in `workspace`, `doc`, `msrv`, and
`aarch64`. Many other jobs remained in progress. The CI run was still in
progress, so GitHub did not provide the failed-job logs at this observation.
These are unresolved findings tracked by #533. CodeRabbit reported success
as a status context but skipped source review because of its file limit.

The user directed this combined geographic delivery to be pushed and merged
after the final round regardless of findings, with all findings moved to a new
issue. This record does not claim the full technical contract passed.
