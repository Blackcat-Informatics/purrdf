# Requirement candidates — issue #491

Lines from the issue body and comments that matched a lexical rule, copied
verbatim with the rule that fired. This is a CANDIDATE list: rules both
over-match (a "should" in passing) and under-match (a requirement stated
without a keyword). The issue-analyzer keeps or drops each row, adds what
the rules missed, and accounts for every silent comment. Code fences are
skipped.

| ID | Source | Line | Rules | Text |
|---|---|---|---|---|
| B1 | body | 33 | list-under-requirement-heading | - `reifiers_of(triple)` is a range lookup (test: equals the filtered full scan; benchmark: sublinear in table size). |
| B2 | body | 34 | list-under-requirement-heading | - With 10^7 reifiers and 6 annotations each, `?r rdf:reifies <<( :s :p ?o )>>` and `?r :producer :x` touch only matching rows (governor evidence: candidates examined proportional to matches). |
| B3 | body | 35 | list-under-requirement-heading | - The same lookups work on `RdfDataset`, packs, paged and segmented datasets (or report their scanning default honestly in EXPLAIN). |
| B4 | body | 36 | list-under-requirement-heading | - Results unchanged across the RDF 1.2 conformance suites. |

## Comments that matched no rule

None.
