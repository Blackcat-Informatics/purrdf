# Requirement candidates — issue #484

Lines from the issue body and comments that matched a lexical rule, copied
verbatim with the rule that fired. This is a CANDIDATE list: rules both
over-match (a "should" in passing) and under-match (a requirement stated
without a keyword). The issue-analyzer keeps or drops each row, adds what
the rules missed, and accounts for every silent comment. Code fences are
skipped.

| ID | Source | Line | Rules | Text |
|---|---|---|---|---|
| B1 | body | 37 | list-under-requirement-heading | - On a dataset with 10^6 reified claims, the estimate for `?r rdf:reifies ?t . ?r :validFrom ?v` is within 2x of the actual row count (today the quoted-triple position is unconstrained). |
| B2 | body | 38 | list-under-requirement-heading | - A paged dataset's plan uses the stored reifier counts (test: plan changes when only reifier counts change). |
| B3 | body | 39 | list-under-requirement-heading | - `?x a :RareClass . ?x :p ?y` is planned class-first when `:RareClass` has few instances and `:p` many, on both `RdfDataset` and paged backends. |
| B4 | body | 40 | list-under-requirement-heading | - A consumer-supplied statistics implementation can be passed in and changes the plan; EXPLAIN shows which statistics were exact. |
| B5 | body | 41 | list-under-requirement-heading | - Summaries from two segments merge to the same statistics as one segment holding both (property test). |
| C1.1 | comment 1 (paudley, 2026-10-15T14:20:21Z) | 6 | modal | and native rule-order caller must consume this seam; an unused statistics API |
| C1.2 | comment 1 (paudley, 2026-10-15T14:20:21Z) | 35 | modal | A real consumer-supplied provider must change an actual plan. Keep cost units, |
| C1.3 | comment 1 (paudley, 2026-10-15T14:20:21Z) | 47 | modal | Run the single required full final gate after source settles. Resolve actual |
| C4.1 | comment 4 (paudley, 2026-10-15T16:45:16Z) | 16 | modal | remain required. No partial PR or whole-issue completion is claimed. |

## Comments that matched no rule

- comment 2 (paudley, 2026-10-15T14:51:42Z)
- comment 3 (paudley, 2026-10-15T15:36:49Z)
