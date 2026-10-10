# Values Insertion work-law correction

Status: source proposal ready; compilation and runtime acceptance NOT MET.

This is one correction for all seven query-trace deltas and the LATERAL ledger failure in `implementation-native-owner-full-check-8.log`, not a golden regeneration. `values-insertion-work-law-correction.patch` is a standard one-home diff against the captured current `binop.rs`; its full `.rs.txt` postimage and source hashes are adjacent. Rustfmt parsed the postimage and `patch --batch --dry-run -p1` passed. No shipping source, build, test, index, reference or forge mutation occurred in this helper lane.

## Observed computation

The existing substitution body still implements Values Insertion as `Join(authored_leaf, singleton_values)`. `expr.rs::join_leaf_with_values` explicitly preserves the original leaf, demotes its **unconstrained** rows to scaffolding and maps the narrowed join as the real source node's row-counting output. Its native clone, box, variable, ground-value and source-map owners remain admitted.

The added `binop.rs::eval_values_restricted_join` changes that real computation. While `in_substituted_exists` is set, it evaluates the singleton right VALUES first and sends those bindings into the left BGP through `finish_seeded_join`/`eval_bgp_seeded`. The leaf now emits only the restricted bag. `eval_join_delivered` adds the same reversal for an actively delivered BGP. A field in the temporary AST is unchanged, but its native probe, candidate visits, committed scaffolding rows, cell peak and governor trip boundary are changed. This is not a source-map bookkeeping defect.

The original `3b6ec4d9f` join body has neither shortcut. Bulk `eval_join` evaluates the left before its ordinary right/join continuation; active `eval_join_delivered` enters `eval_binary_yielding`, which materializes its right and transforms the ordinarily evaluated left. The proposal restores these original execution laws for the inserted VALUES wrapper, using today's admitted native bodies. It does not introduce a metered/resident behavior switch.

The positive-region path is not the culprit: `PositivePlan::region_facts` admits BGP/Join/Union, so a singleton VALUES operand excludes the inserted wrapper. Its existing ordinary seed use for a genuine driver's right positive region remains intact.

## One mechanism covers the complete failure family

The frozen trace records all real leaf/scaffolding work, including rows subsequently narrowed out. Each removed candidate also removes one real committed leaf row. The following accounting is source-backed inference, not a new runtime receipt. Final answers agree already, but exact work and partial/trip evidence must be requalified.

| Case | Frozen / actual fuel | Original versus seeded leaf work explaining the delta |
| --- | --- | --- |
| `existsMinusDomain` | 39 / 37 | Two MINUS-right invocations scan the one excluded quad; only one binding survives the VALUES join. Seeding skips one candidate and one leaf commit: 2 fuel. |
| `existsNestedCorrelation` | 45 / 43 | The nested NOT EXISTS sees the sole `:q` quad once per outer subject; only `:s2` matches the inserted row. Seeding skips the other candidate and leaf commit: 2 fuel. |
| `lateralSharedVarInjection` | 30 / 26 | Three outer subjects ordinarily scan the sole `:q` quad. Only `:a` survives. Two candidate/commit pairs disappear: 4 fuel. |
| `lateralOptionalSubselect` | 86 / 70 | Three outer subjects scan all four labels, then join to their subject. Seeded scans visit four labels total instead of twelve. Eight candidate/commit pairs disappear: 16 fuel. The final padding and 9-cell peak remain unchanged. |
| `lateralSubselectStar` | 72 / 56 | The same twelve-versus-four label work removes 16 fuel. The unfiltered leaf originally holds four two-column rows, giving an 8-cell peak; seeding removes that bag and leaves the outer 6-cell peak. |
| W3C `exists04` | 37 / 33 | Both correlated inner leaf sites have two candidate visits in the frozen ledger and a one-row narrowed result. Seeding removes one candidate/commit pair at each site: 4 fuel; original 4-cell unconstrained leaf becomes a 2-cell bag. |
| W3C `exists05` | 32 / 28 | The same inner leaf work applies under the nested negative EXISTS: 4 fuel and the same 4-to-2 cell reduction. |

`governed_query::ledger_attributes_lateral_rhs_work_to_the_rhs_nodes` independently pins the exact distinction. Its two RHS invocations each scan the sole label quad. The expected RHS source line has six node entries (two leaf, two synthetic JOIN, two VALUES), two BGP candidates, five committed-output fuel units (two leaf + one narrowed JOIN + two VALUES) and **one** row-counting result. Full-check-8 reports six entries, **one** candidate and **four** committed units, still one output row. Removing the shortcut restores actual work rather than charging imaginary compensating fuel.

## Ownership and integration

The diff removes only the shortcut's two dispatches and its implementation. It retains the bounded, typed `TermLookup` and the existing μ-restriction memo. Those avoid repeated reverse-index reads and repeated equivalent EXISTS calls without changing which real quad candidates the original leaf scans or which rows it commits. Success/absence memo entries keep their original native term/control/table grants; source/physical failures remain uncached and typed. Resident callers still create no new lookup cache owner.

The restored leaf bag is produced by the existing `RowsBuilder`/`RetainedRow` BGP body. The ordinary hash join retains its admitted schema, column map, index, collision buckets and rows. No raw resident allocation, new byte multiplier, post-construction fee or detached payload is added. Existing ordinary seeded positive joins, their BGP bound-mask specialization and their native owners remain unchanged.

After the authoritative full-check-8 terminal, the sole writer can apply this one ordinary diff. Qualification must cover the unchanged query `evaluator_trace` (all cases and its frozen fuel sweep), unchanged `ledger_attributes_lateral_rhs_work_to_the_rhs_nodes` and nested ledger neighbor, and unchanged `correlated_owned_admission` seven-query/four-entry matrix under the 16,384 request ceiling and original measured-peak/retained-lifetime assertions. The lookup memo alone has not yet been measured with the restored leaf route, so that traffic acceptance remains NOT MET here. Source/checkpoint failure, governor partial/trip and memo semantic fixtures stay part of the writer's coherent dependent group. No golden, ceiling or ledger assertion changes are proposed.

The earlier `correlated-delivery-lookup-integration.md` described the VALUES seeding shortcut as a correction. This diagnosis supersedes that claim: its **lookup memo** remains useful and qualified by earlier matrices, but the shortcut altered the frozen production work law and must be removed.
