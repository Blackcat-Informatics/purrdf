# Fixed-work arithmetic-source comparison

The standalone Rust workspace imports the original clean bounded XSD source at donor 92ceebafa without editing it, renaming only its evidence package. Both implementations use current shared hash/lex foundations, current testkit counter/report homes and current allocation probe. Its release/bench settings match the repository profiles. This is an arithmetic-source comparison, not a historical shipping binary or complete-query before/after claim. The evidence workspace lockfile records resolved tooling dependencies; it is separate from and does not alter the shipped lockfile.

Inputs reproduce the original small bench's two seeds and 1024 pairs. Fixtures, three warm runs, output destination reservation, answer validation and destruction are outside the acknowledged instruction window. Family/row dispatch selects one function pointer before warming; each measured operation uses that common indirect-call/output-retention boundary. All 1024 returned values are inspected after the window, with exact decimal normalization or full binary64 bits. The four allocation quantities include payload allocated by the operation, keeping returned values live. This boundary is explicit and does not equal the old bench's drop-inside-loop body.

Initial complete source check passed, session 92107 exit 0. Bench-profile build passed, session 28348 exit 0. First collection session 41142 exited 101 during the exact-division warm guard, after 130 complete instruction samples. It is not a completed measurement matrix. Full receipts remain in raw/current-source-counts and current-source-counts.log.

Diagnosis: the historical bounded decimal divider truncates toward zero at scale eighteen (numeric.rs decimal_div_raw), while current DivisionPolicy::default rounds half-even. Of the 1024 input pairs, 506 differ by the last unit; first case is -16.757675715563731671 versus -16.757675715563731672. This is a documented precision-law difference, not evidence of arithmetic corruption. The comparison now explicitly selects scale18/toward-zero for the like-law exact-versus-bounded row. Current public division is unchanged and validated against its half-even exact route. Its report fixture carries a distinct law identity, so the shared comparison API must reject cross-law compatibility. No default or expected answer was weakened to make mismatched laws appear equal.

The first failed root-manifest check preceded writing its target and is preserved separately; it did not compile or measure anything. No failed result is relabeled passed. Original bounded donor and existing compiler/cache/profile settings remain intact. Completed perf endpoints contain no replay data; remove them only once producers exit, retaining every regular receipt.

## Complete matched-law baseline

Precision naming: the fixed fixture/report label half-even18 denotes the current default ExactOrScale law (exact terminating quotient first, otherwise scale eighteen/half-even), not an unconditional scale cap. The workload calls DivisionPolicy::default directly. Toward-zero18 is the explicit Scale(18,TowardZero) comparison with the historical bounded route. Reports with those different labels must not compare.

Final source build session 50764 exits 0. Collection session 74350 exits 0: 21 lanes, ten fresh processes each, complete exact counter coverage, all 1024 answers in every measured process validated; raw/matched-laws-counts contains all 210 full outputs, CSVs and verbose receipts, 21 schema-2 reports and no FIFO remnants. Shared CountReport reader/compatibility/comparison analysis session 26630 exits 0; raw/matched-laws-analysis.log. Different-law public division is correctly rejected as comparable to bounded division. No timing metric is used.

| Operation | Bounded instructions/op | Exact instructions/op | Exact relative change |
|---|---:|---:|---:|
| integer add | 89.4062 | 85.4062 | -4.474% |
| integer multiply | 119.4062 | 109.4072 | -8.374% |
| decimal add | 202.3408 | 178.9902 | -11.540% |
| decimal multiply | 170.4062 | 194.7305 | +14.274% |
| decimal divide, toward-zero18 | 250.9365 | 374.5312 | +49.253% |
| parse/render | 1770.3447 | 2579.1909 | +45.689% |
| decimal to binary64 | 130.7344 | 149.3594 | +14.246% |

These counts include common indirect-call/retained-output overhead and are not numerically interchangeable with the original setup-subtracted table. The current public value path is separately measured in the same reports; public division uses half-even18 and has no like-law bounded row. This baseline establishes remaining work, not issue completion or complete-query performance.

All 21 allocation windows also exit 0, with every full answer checked, raw/matched-laws-allocations. All non-render rows report zero in all four quantities. Parse/render bounded: 3952 calls, 55473 requested bytes, 18121 retained, 18133 peak. Exact and public each: 1966 calls, 24687 requested, 16777 retained/peak. These totals are per 1024 returned values with the destination Vec reserved before measurement, not per call or whole-query figures.

The concrete next production change reuses PreparedDecimalText's existing exact length and write_to instead of growing a fresh String through generic formatting. A focused public allocation control is being run on the unchanged implementation before applying that change. All remaining issue criteria stay active.

## Reserved-output result

The unchanged new allocation assertion fails on original source (session 91219 exit 101: zero requests eight output bytes versus one) and passes after output reservation (session 79265 exit 0). Complete native exact/governance runs 35+7 cases, session 34630 exit 0. Actual packaged WASM executes the new allocation control and all seven frozen/oracle/large exact cases, sessions 54168 and 9368 exit 0. Existing successful byte digests are unchanged.

Post-change comparator build session 51137 and complete 210-sample collection session 63822 exit 0. All 210 full outputs have 1024 numbered checked answers. Every counter sample passed the shared strict unscaled/exact enabled-running coverage law. All 21 allocation windows pass. Shared CountReport compatibility/comparison runs 21 same-family before/after comparisons, session 74826 exit 0; raw/reserved-output-analysis.log.

Exact parse/render instructions/value: 2579.1909 to 2228.2861, -13.60523%, bootstrap interval [-13.60783%,-13.60486%]. Public value path: 2593.1870 to 2242.2852, -13.53168%, interval [-13.53193%,-13.53147%]. All other comparison intervals contain zero or differ only inside the recorded small noise band; retain their exact observations. No global no-regression or full parity assertion.

Per 1024 exact or public parse/render values, allocation calls 1966 to 1024, requested bytes 24687 to 13359, retained and peak bytes 16777 to 13359. The bounded route remains at 3952 calls, 55473 requested, 18121 retained and 18133 peak. This changes only canonical destination reservation and reuses the original prepared digit writer. The exact path still exceeds bounded 1770.3462 instructions/value, leaving full parity unfinished. No whole-query, large-curve or SIMD speed claim follows.

Strict all-target XSD Clippy session 3204 exits 0. Complete helper hygiene session 57980 exits 0, including census 85 enforced jobs, 117 distinct rows, 2452 source files and 141 domains; no exemption or new implementation home. Emitted assembly reconciliation is recorded separately and does not require replaying unchanged qualified kernels.
