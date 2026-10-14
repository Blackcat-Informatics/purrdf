Nested FILTER/EXISTS and correlated LATERAL exhausted the optimized WASM stack.
Their children now suspend on the admitted evaluator worklist, retaining copied
pattern owners and restoring scopes before resuming the original computation.

Published production passes the full local gate, all three original optimized
depth controls, hosted449/449 npm cases and local Worker acceptance. Ordinary
rows remain40 bytes. Corrected native admission/refund/final-owner controls pass3/3;
the revised scalar physical-stack fixture passes1/1. Required commit hooks run
normally. Performance claims exclude timing and noisy instruction comparisons.

Remaining native/SIMD, final-source assembly identity and complete hosted
qualification are disclosed and retained in [follow-up545](https://github.com/Blackcat-Informatics/purrdf/issues/545).
They are not represented as passing. The SIMD fixture build failed during an
installed-compiler transition; no tool override or cache workaround was used.

Closes #531.
