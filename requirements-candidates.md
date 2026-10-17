# Requirement candidates — issue #590

Lines from the issue body and comments that matched a lexical rule, copied
verbatim with the rule that fired. This is a CANDIDATE list: rules both
over-match (a "should" in passing) and under-match (a requirement stated
without a keyword). The issue-analyzer keeps or drops each row, adds what
the rules missed, and accounts for every silent comment. Code fences are
skipped.

| ID | Source | Line | Rules | Text |
|---|---|---|---|---|
| B1 | body | 3 | modal | The downstream authored-integrity runtime must evaluate canonical equality/distinctness over original RDF values. Its actual source-to-enactment regression joins two distinct IRI metadata values but emits no violation, because the compiler currently treats termDistinct as stored data. ORDER BY compare_values is explicitly the wrong semantic relation. Copying the SPARQL equality implementation or rendering values into query text would duplicate authority and degrade the dataflow. |

## Comments that matched no rule

None.
