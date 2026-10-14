# Numeric prior-art assessment

VERDICT: useful donor components, no qualified complete delivery.

Read the current main implementation and both preserved clean donors. The
authoritative Stage brief names closed #422/#423 and one related forge result.
Its last200-commit trailer tally is none25, semantic-identity-loss2, performance2;
these are repository classifications, not a count of identical numeric failures.
Recurrence of this exact defect is UNKNOWN. No new broad history campaign is
needed. No donor files were changed or deleted and no tests were run.

## Exact-default donor: v4-decimal-defaults,5854ef59e

This is a single explicit work-in-progress commit over b0ed2f8c5:37 paths,
3760 additions/4309 deletions. The live preservation notice explicitly says
unreviewed/in-progress, not finished. Five encoded comments were not decoded.

| Concrete donor | Reuse | Required reconciliation |
|---|---|---|
| exact/rounding.rs:94–155 ExactOrScale plus parser/Display | Actual default exact-or-18:half-even, existing explicit policy spellings; reusable semantic design and text tests | Port into current one policy home and all current exhaustive consumers; retain source/host default capture and errors. |
| exact/decimal.rs div exact-small/fallback, reduced factors2/5 | Actual small termination classification and only NonTerminating-triggered rounded fallback; useful algorithm | Current decimal has try_*_admitted, fallible storage and NumericOperationLayout; donor's direct unbounded constructors cannot replace these. Preserve physical refusal rather than treating it as nontermination. |
| value.rs Integer{exact::Integer}/Decimal(exact::Decimal), numeric.rs and numeric/exact_path.rs | Removal of duplicate bounded public variants and conversions, useful caller migration inventory | Current numeric.rs includes admitted render/layout and scratch-controlled operations absent from donor; selective port only. Preserve Float/Double promotion and exact integer subtype semantics. |
| expr/modifier/stat_agg/list_fn, SHACL/ShEx/OWL/CDT/OKF callers | Concrete changed call sites show required exact wrapper extraction/borrows and aggregate totals | Current evaluator evidence, candidate scopes, geometry profile and native ownership must remain. Do not replace whole expr/modifier files or duplicate numeric homes. |
| temporal.rs + temporal/seconds.rs | Public exact accessor adaptation and consumer inventory only | **Not fully exact seconds**: private Fixed{i128,scale:u8},MAX_DECIMAL_SCALE18 remains explicitly documented and parsed. Reusing this storage violates #470 item3. Replace actual seconds storage/arithmetic with the current exact admitted home. |
| numeric/tests.rs, exact_tower/exact_values/governance/wasm fixtures, vector generator | Rich signed division, promotion, IEEE-rounding and identity regressions; selected cases useful | Reconcile intentional default changes, current first-party harness and frozen-corpus policy; regenerate only authorized generated exact fixtures from actual generator, preserve independent oracle expectations. |
| exact/cost.rs branch-shape estimates | Maps additional exact-or-scale branches and expansion shape | Obsolete compared with current detailed scratch layout/cost model. Reprove current observed work/storage bounds, including termination probe plus rounded fallback. |

The donor does not contain the complete current CLI/Python/C/WASM default
migration, governor version/vector migration, Breaking/book/zh-Hans updates or
required performance tables. Its older32-bit limb Karatsuba implementation
and threshold64 cannot be transplanted into main's u64/three-inline-limb layout.

## Bounded donor:422-bounded-numerics,92ceebafa

Five nonmerge commits remain on this historical branch, covering bounded
lexical/typed-error correctness and exact comparisons across host consumers.
The issue is already closed; Git ancestry alone does not imply its squash-landed
behavior is absent from main. Current main already contains exact tower, numeric
comparison, subtype/error and aggregate infrastructure; compare individual
behavior before salvaging anything.

Useful concrete regression populations include sparql-eval/tests/bounded_numerics.rs:
comparison past i128 (217), >18 fractions (250), equality (286), ORDER BY/MIN/MAX
(315), casts (386), binary conversion (419), AVG=SUM/COUNT (583) and composite
elements (721). xsd/tests/bounded_numerics.rs carries exact digit/order/float
oracle properties; shex/tests/exact_facets.rs exercises large exact facets and
imports. CSVW, CLI and Python consumer cases provide actual entry-point inventory.

Do not transfer expectations that mandate old precision truncation or numeric
overflow merely because the result exceeds i128/18places (e.g.evaluator case601
and XSD cases40/65/93). The new exact-only contract supersedes that behavior.
Preserve genuine invalid-lexical, zero-division, XSD subtype and resource errors.
Do not resurrect DecimalDigits or a second bounded arithmetic implementation
where current exact consumers already provide the job. Old external property
testing imports must use the current native testkit harness if salvaged.

## Main infrastructure and missing optimization work

Main's exact Integer already uses Small(i128)/Big, canonical derived Eq/Hash;
BigInt stores three inline u64 limbs, pooled/shared original ownership and bounded
LimbScratch. Karatsuba lives in bigint/compat.rs at32limbs; bigint.rs owns native
magnitude operations, normalized long division and repeated small-base decimal
rendering. exact/cost.rs, exact/decimal.rs and rational/{certified,geometry}.rs
contain recent geo admission work. Reuse these homes, the alloc probe, fixed-work
exact benches, query_eval arithmetic/order/aggregate/filter lanes, frozen exact
vectors and existing architecture dispatch/assembly machinery.

Neither donor supplies demonstrated Toom/FFT-or-NTT, subquadratic division or
radix conversion, new SIMD acceptance, rational optimization or current seven-row
parity. Those remain implementation and measurement work, not donor salvage.
No external codebody/coefficient import or dependency is justified by this review.

The sole new implementation checkout is #470. Transfer proven unique donor units
there, retain baseline evidence needed for #443, then delete donors only after
verified transfer. Coordinate shared core/lex/evaluator/host changes with S1;
numeric-only xsd algorithms can advance independently. The actual numeric cost
and public API migration must settle before final execution-worker comparisons.
