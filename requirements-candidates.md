# Requirement candidates — issue #536

Lines from the issue body and comments that matched a lexical rule, copied
verbatim with the rule that fired. This is a CANDIDATE list: rules both
over-match (a "should" in passing) and under-match (a requirement stated
without a keyword). The issue-analyzer keeps or drops each row, adds what
the rules missed, and accounts for every silent comment. Code fences are
skipped.

| ID | Source | Line | Rules | Text |
|---|---|---|---|---|
| B1 | body | 11 | modal | These failures were observed on PR #535 head `96f8c5f63`, whose source delta is confined to packaging scripts and Makefile comments; both named Rust files are unchanged from its main base. PR #535 was rejected and closed without merging. Its discarded tooling changes must not be revived. The hosted job logs are the source of these observations, not evidence that their fixes have been implemented. |

## Comments that matched no rule

- comment 1 (paudley, 2026-10-12T05:01:27Z)
