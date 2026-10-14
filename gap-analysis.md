# Exact numeric delivery: current remediation

Owner assessment. No child agents are used. This is not an independent review.
The original issue and all five host requirements remain in scope. Earlier
independent reports retain their recorded source and scope.

The repaired full local gate passed on published `0e9a3d9c4`, including release
WASM. The refreshed optimized package passed 458 tests, TypeScript and packed
installation; 42 installed Python controls passed. The earlier failed gates
remain attributed failures. Those passes do not discharge hosted findings.

| Finding | Production correction or required action | Verification |
| --- | --- | --- |
| Public bounded AVG API survived the exact-only migration | Remove both unused public AVG helpers and their exports. Preserve historical implementations privately under `cfg(test)` for the existing differential controls. Actual AVG uses `numeric_div_with_policy`; no shipping caller uses either removed helper. | Affected all-target compilation and public rustdoc pass. Existing complete host and AVG receipts apply to unchanged callers. |
| Three stale public rustdoc links | Describe exact decimal canonicalization and admitted temporal coefficient/scale comparison; remove the obsolete evaluator lexical-bypass claim. | Warning-denied public XSD and evaluator documentation pass. |
| Three needless closure borrows found by current hosted Clippy | Pass the existing copyable closure directly. No computation or cost changes. | Warning-denied affected all-target Clippy passes. |
| CodeQL labels a test byte counter as sensitive `account` data | Name the actual fixture `ScratchBudget` and its variable `allocation_budget`. It holds only live, peak and limit byte counts. Preserve every admission/refusal/release assertion. No analyzer suppression or security-policy change. | The original temporal peak/refusal/release test passes. Hosted alert readback remains required. |
| Generated baseline join count says 43, hosted assembly measures 42 | Use the existing complete seven-configuration projection producer; review its generated patch without changing floors, targets or arithmetic restrictions. | Six hosted configurations passed. Baseline floor passed; parity failed. Complete new projection and ordinary hosted parity remain required. |

No issue closure, merge readiness or fresh independent approval is claimed.
Current inline feedback is retained in `tasks/current-inline-reviews.json`.
Source changes must receive normal hooks and publication, the generated patch
must be reviewed, and remaining hosted findings must be resolved before merge.
