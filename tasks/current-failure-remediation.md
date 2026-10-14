# Six-target final-gate remediation

The original full gate failed; its log and failure index remain unchanged.

| Target | Finding and correction | Current check |
|---|---|---|
| Evaluator library: calendar constructors | Admitted rendering inferred dateTime from the value and erased requested dateTimeStamp. The existing render home now takes the literal datatype explicitly; aggregate/numeric callers supply their original datatype. Workspace-bound error behavior is preserved. | All seven cast controls pass, including the 21-place stamp and complete exact-binary decimal oracle. |
| Evaluator library: float decimal cast | Oracle correctly expected the exact binary value, but still required a bounded variant for short payloads. Full lexical oracle remains; all decimal casts use the one exact variant. | Included in the seven passing cast controls. |
| Shapes temporal order | Regex capped fraction length at 18 and interpreted time's 24:00 as the next day. Exact fractions are admitted; time aliases midnight, while dateTime retains day carry. Extended controls exposed repeated-prefix expansion: one pattern exceeded the unchanged 1 MiB grammar limit, and both exceeded the default matcher's 10 MiB compile budget. The shared fraction home factors prefixes. Grammar equivalence uses the existing syntax parser; consumer matcher admission remains bounded. | All three grammar, three numeric-order and four temporal-order controls pass with 21-place neighboring/equal values. Actual schema/validator parity passes after correcting the obsolete fixture's treatment of time midnight. Failed size and stale-fixture runs remain failed. |
| Parser AST snapshot | Two new governor queries were absent from the projection. | Original generator added exactly four lines for the two new requests; previous rows unchanged. Ordinary replay passes. |
| CDT lexical scan | New numeric-default.rq/.ttl and temporal-exact.rq add three scanned inputs. The observed population is 4,122. | All four ordinary controls pass; other coverage counts are unchanged. |
| Evaluator trace | Ten current numeric/default and AVG cases have changed fuel under profile 18; complete answers/cells/scratch retain their prior values. | The original generator and ordinary query/update replay pass. Every changed line is a receipt, node charge or fuel sweep. The complete owner delta assessment is `current-trace-and-proof-assessment.md`. |

The current focused jobs are source-bound to these repairs. They do not establish
fresh package, installed-host or whole-gate acceptance. Required remaining
qualification follows the terminal focused results; no hook or acceptance floor
is bypassed, and no finding is moved to another issue.

Current logs: `current-cast-family-regressions.log`,
`current-temporal-size-diagnosis.log`, `current-exact-order-compaction.log`,
`current-regex-grammar-admission.log`, `current-numeric-order-compacted.log`,
`current-temporal-order-compacted.log`, and `current-emitter-temporal-parity.log`.
The size diagnosis is retained. Its temporary matcher-error probe is removed;
the assertion retains the pattern byte count as an actionable failure message.

The three shared regex emission/admission controls now pass, including rejection
of invalid and oversized source, the existing Unicode equivalence cases and the
valid-grammar/default-matcher-memory distinction. All focused chains are terminal;
the numeric repaired full gate is session 42009. The original full gate remains
failed. The current helper gate passes with unchanged proofs, authority paths and
acceptance criteria after the constructor aggregate refresh.
