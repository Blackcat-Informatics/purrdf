# Requirement candidates — issue #702

Lines from the issue body and comments that matched a lexical rule, copied
verbatim with the rule that fired. This is a CANDIDATE list: rules both
over-match (a "should" in passing) and under-match (a requirement stated
without a keyword). The issue-analyzer keeps or drops each row, adds what
the rules missed, and accounts for every silent comment. Code fences are
skipped.

| ID | Source | Line | Rules | Text |
|---|---|---|---|---|
| B1 | body | 21 | list-under-requirement-heading | - Metadata validation covers all five tables and both compression modes with |
| B2 | body | 23 | list-under-requirement-heading | - Blob reconstruction at 1 KiB and 1 MiB retains the original pointer and complete |
| B3 | body | 25 | list-under-requirement-heading | - Actual malformed/checksum and structured-cause controls preserve original |
| B4 | body | 27 | list-under-requirement-heading | - All ten frozen file identities remain unchanged. Native library/integration, |
| B5 | body | 28 | modal | strict affected compilation, required gates and release WASM portability pass. |
| B6 | body | 30 | list-under-requirement-heading | - Normal signed hooks, current-head independent hosted review and complete CI |

## Comments that matched no rule

None.
