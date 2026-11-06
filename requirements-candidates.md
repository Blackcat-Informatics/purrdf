# Requirement candidates — issue #1107

Lines from the issue body and comments that matched a lexical rule, copied
verbatim with the rule that fired. This is a CANDIDATE list: rules both
over-match (a "should" in passing) and under-match (a requirement stated
without a keyword). The issue-analyzer keeps or drops each row, adds what
the rules missed, and accounts for every silent comment. Code fences are
skipped.

| ID | Source | Line | Rules | Text |
|---|---|---|---|---|
| B1 | body | 20 | modal | - Preserve the last fully accepted checkpoint, rollback, poisoned-builder refusal and original ingestion receipts. A poisoned builder must never publish its accepted subset as success. |
| B2 | body | 26 | checkbox | - [ ] Actual source-read, readiness, ingestion, writer and blob-decoding failures expose their native fields through the existing visitor. |
| B3 | body | 27 | checkbox | - [ ] Failed ingestion preserves rollback and poisoning, including later emission refusal. |
| B4 | body | 28 | checkbox | - [ ] Existing successful composition/projection cases retain their output and identity behavior. |
| B5 | body | 29 | checkbox | - [ ] Affected callers compile, focused native/host checks pass, and signature/ownership migration is documented. |
| B6 | body | 37 | modal | Use focused native behavioral checks and affected production-consumer compilation with the required profile/warnings. WASM execution is limited to documented dispatch, host-interface, shadow-stack or frozen-identity obligations. Required hooks and release gates remain mandatory. Do not add compiler-guarantee tests, duplicate semantic suites, or a whole-workspace run merely to finish this child. |

## Comments that matched no rule

None.
