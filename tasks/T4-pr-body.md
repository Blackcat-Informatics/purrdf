Native CI and C tests now retain source-bound phase, Cargo freshness, runtime and timing receipts through shared Rust tooling and an optional same-runner cold/warm campaign. Comparison keeps workspace/first-party targets and features exact, requires every baseline external dependency variant, and explicitly reports extra partition dependency builds with their full costs.

The completed twelve-case/24-receipt campaign demonstrates the existing nested C profile fix:32.88% less cold command time and24.22% less unchanged-warm command time. Six native shards cost more aggregate work; their59.17%/54.37% lower idealized max is a model, not observed parallel speedup. Actual production scheduling observations are reported separately. Normal tests, strict O3/assertions/overflow, C ABI controls and downstream coverage remain intact.

Validation: full local make check passed on measurement source47c9; the comparison-only correction passed36 owning tests, strict all-target CAPI clippy, normal hooks and the original Rust validator against all24 immutable measured receipts. The original hosted comparator failure is retained; corrected replay passes without rerunning or altering measured workloads. Current-head PR CI and review are pending.

Closes #308.
