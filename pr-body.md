Segmented quad, reifier and annotation scans now retain their authenticated row block and validated record range across yields. Each row decodes directly from that retained allocation; readiness, cache lookup and record-header validation occur at block boundaries.

Nested scans can evict a retained row's cache entry while the iterator keeps the original bytes and live-byte grant. Dictionary term pins keep their existing eviction refusal. The old row owner is released before admitting the next block and on early iterator drop.

Validation: two deterministic block-operation/ownership/fault controls and all 33 existing segmented/session/layered consumer tests pass. Strict affected core library/test Clippy passes. No timing measurements were used. Final hosted workspace and portable checks are required before merge.

The capacity-proof ledger refresh is required by the repository's source-bound gate. Its constructor identity includes the complete visible core module domain, including the changed session and cache files. The original gate rejects the stale binding. The fresh standard index confirms all 40 proof rows keep their authority sources, sinks, classes, lines and reasons; only their shared constructor binding changes. Helper hygiene then passes. No checker or exemption is relaxed.

Closes #563.
