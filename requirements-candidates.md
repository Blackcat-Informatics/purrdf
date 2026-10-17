# Requirement candidates — issue #538

Lines from the issue body and comments that matched a lexical rule, copied
verbatim with the rule that fired. This is a CANDIDATE list: rules both
over-match (a "should" in passing) and under-match (a requirement stated
without a keyword). The issue-analyzer keeps or drops each row, adds what
the rules missed, and accounts for every silent comment. Code fences are
skipped.

| ID | Source | Line | Rules | Text |
|---|---|---|---|---|
| B1 | body | 20 | modal | Update a `TextIndex` by a delta of added and removed literals instead of rebuilding it. BM25F's statistics (document count, field lengths, document frequencies) are sums, so a delta update is exact; the ranking law and `Fixed` arithmetic stay as they are, and the result should be identical to a fresh build over the same dataset (a good conformance oracle). |

## Comments that matched no rule

None.
