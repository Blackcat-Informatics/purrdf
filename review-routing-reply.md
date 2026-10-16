Fixed in `ee13e8327`, with the portable regression and main integration carried
through `a94af3ab2`. Only a `Residency` refusal for optional routing-index growth
skips the hint. The admitted payload and exact request remain usable; source,
allocator and mandatory payload/request failures retain their original latch.
`cache_insert` receives the admission decision and cannot add an unreserved hint.

The original pressure reproduction fails at 2,006,315 requested bytes against a
2,000,000 ceiling. The correction passes the original affected native controls
and the public pressure case natively and on scalar/SIMD WASM. The combined
profile-2 source also passes all 18 segmented and 15 stack cases, all 21 affected
units (including the corrected long-fixture lookup rerun), and the unchanged
eight-lane 2,000/200,000-claim LIMIT/key/COUNT workload under 256 MiB. Strict and
fresh helper checks pass. Current-head hosted qualification is running; no
unexecuted portable configuration or independent owner approval is claimed.
