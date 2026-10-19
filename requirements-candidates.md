# Requirement candidates — issue #494

Lines from the issue body and comments that matched a lexical rule, copied
verbatim with the rule that fired. This is a CANDIDATE list: rules both
over-match (a "should" in passing) and under-match (a requirement stated
without a keyword). The issue-analyzer keeps or drops each row, adds what
the rules missed, and accounts for every silent comment. Code fences are
skipped.

| ID | Source | Line | Rules | Text |
|---|---|---|---|---|
| B1 | body | 20 | modal | - A query or rule containing `NOW()` must be reproducible. Katamari will pass the instant it means; the answer has to be computed at that instant, and re-running it must give the same result. |
| B2 | body | 33 | list-under-requirement-heading | - A query with `NOW()` run twice with the same `with_now` value returns identical results, and differs when the value differs; without the option, behaviour is unchanged. |
| B3 | body | 34 | list-under-requirement-heading | - For a stack with generations G1..G5, `snapshot_at(3)` answers every query exactly as an eager seal of G1..G3 (with their removals) would. |
| B4 | body | 35 | list-under-requirement-heading | - A valid-time range query over 10^7 reified claims reads only the pages whose summaries overlap the range (G8 evidence), with results identical to the post-filter plan. |

## Comments that matched no rule

None.
