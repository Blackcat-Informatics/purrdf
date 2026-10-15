# Requirement candidates — issue #551

Lines from the issue body and comments that matched a lexical rule, copied
verbatim with the rule that fired. This is a CANDIDATE list: rules both
over-match (a "should" in passing) and under-match (a requirement stated
without a keyword). The issue-analyzer keeps or drops each row, adds what
the rules missed, and accounts for every silent comment. Code fences are
skipped.

| ID | Source | Line | Rules | Text |
|---|---|---|---|---|
| B1 | body | 30 | modal | - `crates/lex/src/allocation.rs:503-526` (`Memory::reserve`): when `required > capacity`, it does `try_reserve_exact(required)` into a new `Vec` and then `replacement.append(values)`. That's a full copy at every growth step, growing by exactly one element. |
| B2 | body | 36 | modal | - Geometric growth inside `Memory::reserve`, e.g. reserve `max(required, 2 * capacity)`, admitting the larger layout through the same accounting. The admission stays exact; the growth step stops being one element. |
| B3 | body | 37 | modal | - Separately: should `LIMIT 10` over a single pattern need to intern every term the scan passes? A streaming page that touches about 10 representatives would make first results independent of image size. |
| C1.1 | comment 1 (paudley, 2026-10-14T19:45:32Z) | 47 | modal | Run focused affected consumers during implementation and the required final gate |
| C1.2 | comment 1 (paudley, 2026-10-14T19:45:32Z) | 51 | modal | allocation changes must preserve existing exact-capacity conversion and ownership |

## Comments that matched no rule

- comment 2 (paudley, 2026-10-14T19:50:39Z)
- comment 3 (paudley, 2026-10-14T20:01:57Z)
- comment 4 (paudley, 2026-10-14T21:32:16Z)
