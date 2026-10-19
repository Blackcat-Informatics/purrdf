# Requirement candidates — issue #570

Lines from the issue body and comments that matched a lexical rule, copied
verbatim with the rule that fired. This is a CANDIDATE list: rules both
over-match (a "should" in passing) and under-match (a requirement stated
without a keyword). The issue-analyzer keeps or drops each row, adds what
the rules missed, and accounts for every silent comment. Code fences are
skipped.

| ID | Source | Line | Rules | Text |
|---|---|---|---|---|
| B1 | body | 36 | modal | Output must be byte-deterministic, with no timestamps. Proposed |
| B2 | body | 43 | checkbox | - [ ] `vectors/shacl12/` (and `vectors/shacl/` if it has drifted) is vendored |
| B3 | body | 46 | checkbox | - [ ] Every approved 1.2 and 2017 entry passes against its approved |
| B4 | body | 49 | checkbox | - [ ] The harnesses emit deterministic EARL for both suites, and harness tests |
| B5 | body | 51 | checkbox | - [ ] Both EARL files load in the WG reports pages with no page errors, no |
| B6 | body | 53 | checkbox | - [ ] Both reports are generated from a tagged release and submitted upstream, |

## Comments that matched no rule

None.
