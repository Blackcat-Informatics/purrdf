Task3 committed/pushed6641041e4: optional hosted profiling over all12 comparison
cases, unchanged required gates, observed compiler/tools/hardware/concurrency,
cold/warm controller execution and failure-preserving evidence capture.

Independent review caught cross-attempt artifact collisions/selection. Corrected
run+attempt identities and preflight restoration preserve old evidence, refuse
stale/mixed/partial/alias inputs and require a full workflow rerun. Independent
re-review PASS;22 focused tests, strict all-target clippy, actual controller build
and wrong-attempt refusal, workflow/shard/profile/parity/fmt checks PASS. Normal
configured commit hooks passed.

Actual matched campaign, source/config invalidation witnesses, attributable
reduction and full qualification remain required in Tasks4–5. No measured
speedup, hosted campaign, PR or merge is claimed.
