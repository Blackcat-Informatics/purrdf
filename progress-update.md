The complete fallible evidence and scoped retrieval implementation is published
in PR #684. It uses the original dataset, evaluator, admission owner, evidence
source, producer streams and fusion; operational errors cannot become unknown
evidence or successful partial output.

The first full hosted run exposed a legacy host test that explicitly expected an
unbounded non-latching error to become a rendered query diagnostic. Commit
bbfd189ca strengthens that assertion to require the exact original operational
error across both resource modes and numeric/temporal certificate cases. All six
host cases pass natively and on scalar/SIMD WASM, strict checking and normal
hooks pass, and the correction is pushed. Production source is unchanged from
the independently reviewed implementation.

Current-head hosted qualification is pending. This issue is still open and is
not complete until the checks pass, feedback is settled and the PR is merged.
