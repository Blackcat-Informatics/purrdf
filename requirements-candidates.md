# Requirement candidates — issue #597

Lines from the issue body and comments that matched a lexical rule, copied
verbatim with the rule that fired. This is a CANDIDATE list: rules both
over-match (a "should" in passing) and under-match (a requirement stated
without a keyword). The issue-analyzer keeps or drops each row, adds what
the rules missed, and accounts for every silent comment. Code fences are
skipped.

| ID | Source | Line | Rules | Text |
|---|---|---|---|---|
| B1 | body | 7 | modal | Use one first-party generic ownership implementation in the existing common dependency home. Move both production callers onto it, remove both private duplicate enums/accessors, and preserve inline owning storage, zero-copy borrowing, immutable lifetime exclusion and ordinary move/drop behavior. The wrapper must impose no Clone or ToOwned requirement and no new allocation for borrowing. Do not change the census, add an exemption, box or clone the prepared knowledge base, add a shipping dependency, or introduce a semantic feature. |

## Comments that matched no rule

None.
