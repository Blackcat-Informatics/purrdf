# Numeric delivery issue analysis

Scope: complete #443 and #470 contracts, live bodies read, #443 has no comments.
For #470 the preservation notice (comment6029995277) was read; five encoded patch
comments were deliberately not decoded, per assignment. The existing donor
checkout is the source. Stage brief's six-comment coverage is therefore accounted
for. Current implementation checkout starts from main dd79ef77e. No production
edits, builds or tests were performed. Findings are source observations and
acceptance design, not implementation qualification.

Repository `.baseline`, `.goals` and AGENTS require first-party Rust, one home,
exact ownership, portable behavior, truthful results and normal hooks. The
emergency ledger cannot be used for authorized scope cuts. No governing ADR is
cited by these issue bodies. Prior geo/evaluator permission to ship their failed
final rounds does not silently reduce these numeric contracts. The explicit
removal of exclusive timing campaigns is respected; #443 itself requests
instructions:u measurements, not an exclusive wall-time campaign.

## Complete contract and concrete acceptance

| Requirement | Current production home / gap | Task and falsifiable acceptance |
|---|---|---|
| 443: parity for all seven original rows | xsd/benches/exact.rs contains integer add/mul; decimal add/mul/div; parse+canonical render; decimal→f64 exact/bounded pairs. Decimal still canonicalizes in exact/decimal.rs. Historical figures are not current measurements. | T3: unchanged fixed input populations and operation law, instructions:u per completed operation including loop control; all seven candidate rows no worse than bounded baseline. Preserve baseline before removing its API. Original table:49.7/66.4,78.0/95.4,138.7/131.2,158.9/105.4,346.5/231.7,1469/1459,100.4/97.8 (exact/bounded). |
| 443: actual winning tower dispatch | xsd/numeric.rs retains bounded Decimal/i128 and Big variants, numeric/exact_path.rs promotion; evaluator expr/modifier/stat_agg consume these. | T3: actual SPARQL arithmetic, ORDER BY, SUM/AVG and FILTER query lanes, same answer bags and explicit policy, before/after instruction tables with no regression. Internal small machine representation remains allowed; do not restore a second public bounded API after T1. |
| 443: Toom and very-large transform multiplication | bigint/compat.rs currently selects schoolbook/Karatsuba at32 native u64 limbs; donor older64-u32 threshold is not directly reusable. | T4: Toom-3 above Karatsuba and exact FFT/NTT-class very-large multiply, native/wasm32 instruction threshold sweeps, balanced/unbalanced operands and cutoff neighbors, large curves and exact product vectors. Toom-4 is optional; Toom/large transform coverage is not satisfied by existing Karatsuba alone. |
| 443: subquadratic division | bigint.rs mag_div_rem/divide_normalized still uses normalized long division. | T4: BZ or Newton-style divide; signed q*d+r=a and abs(r)<abs(d), near-equal/asymmetric lengths, zero/refusal law, curves versus multiplication and threshold sweeps. |
| 443: subquadratic decimal conversion | bigint.rs from_decimal_bytes_using and repeated CHUNK_BASE division in prepare_decimal_digits_using are current homes. | T4: divide-and-conquer parsing/rendering with cached powers;40–40000-digit and transition populations, exact canonical bytes and instruction slopes, cache construction charged and owned. |
| 443: architecture SIMD | Existing dispatch/manifest are reusable infrastructure, not new kernel proof. | T5/6: x86_64-v3/v4, NEON, wasm simd128 candidate limb loops with portable reference, frozen goldens and registered emitted sites; actual execution on applicable architectures and measured winning/useful paths. Preserve scalar fallback. |
| 443: medium allocations | Integer already inlines i128; BigInt already inlines three u64 limbs and has LimbScratch. | T5: actual alloc-probe counts and requested/live/peak bytes for medium chains/division before/after; reduction must be demonstrated beyond existing inline capacity. Original allocation owners must survive clones and release at last owner. |
| 443: rational reduction | exact/rational.rs constructs gcd-reduced form and chained arithmetic; bigint gcd remains the reusable kernel. | T5: cheaper binary/Lehmer-class reduction and reduced redundant chained reductions, exact numerator/denominator/canonical equality and hash parity, instruction/allocation evidence. |
| 443: sound non-over-refusing costs | exact/cost.rs now contains detailed NumericOperationLayout, admitted scratch destination calculations and shape-specific costs. | T4–6: estimates bound actual work/peak storage for every algorithm/threshold; sufficient declared budgets admit representative supported cases, one-below actual resource boundaries refuses cleanly. Do not merely replace costs by huge safe ceilings. |
| 443: all supporting constraints | Existing exact oracle, frozen exact_numeric_vectors, exact_wasm_determinism, allocator probes and bench harness. | T3–6: bit-identical native/wasm results, no new external dependency, each changed computation has bench row plus BENCHMARKS/SIMD documentation; no dependency removal or speed claim based only on microbenchmarks. |
| 470: exact-only integer/decimal API | numeric.rs public bounded Decimal and parse_integer→i128; XsdValue bounded/Big representations; temporal fields consume bounded Decimal. | T1/2: one public exact Integer/Decimal/Rational representation; all ordinary and admitted readers/operators/casts/aggregates updated. Rust callers exercise values beyond i128 and >18 fractional places. Internal machine fast paths and genuine XSD subtype constraints remain valid. |
| 470: exact-or18-half-even default | exact/rounding.rs xsd_default currently Scale18/TowardZero. | T1/2: 1/(2^80) preserves its full terminating expansion,2/3→0.666666666666666667 and-2/3 symmetric; divisor reduced against numerator before termination test. Preserve exact nontermination refusal; explicit N means existing truncation and N:ROUNDING retains ties, e.g.1/8 at2:half-even→0.12. |
| 470: fully exact temporal seconds | temporal.rs seconds, duration arithmetic and accessors use bounded Decimal. Donor hides same bound behind Fixed. | T1/2: dateTime/time/duration parse/render/comparison/arithmetic preserve nonzero fractions beyond18 places, e.g.PT0.0000000000000000001S, and large representable seconds; no silent truncation or private18-place cap. Separate calendar-range constraints from seconds precision. |
| 470: every host | RustQueryOptions, CLI --division, Python query/prepared/update interfaces, C dataset policy setter/query, WASM QueryEngine default and captured async policy. | T2/6: installed C/Python and packed optimized WASM plus CLI execute shared examples above, getters/default omission/explicit overrides/error mapping; async jobs retain start-time policy. Include query, prepared, governed and UPDATE WHERE consumers where policy is forwarded. |
| 470: public identities and docs | Main governor v16 just integrated geo/R2; frozen governor/trace corpora, book/zh-Hans, CHANGELOG. | T2/6: next noncolliding profile version, actual schedule/charge vectors regenerated through existing generators and reviewed; historical identities retained. Breaking migration documentation and signatures match exact-only APIs and every host's policy spelling. |

## Important consequences

Lazy canonicalization is an issue suggestion, not permission to break derived
Eq/Hash: exact Decimal currently derives them from canonical coefficient/scale.
Either preserve canonical storage or redesign all compare/hash/render/equality
consumers coherently and prove value identity. Compare alternative strategies by
the required instruction rows before committing a representation campaign.

Division benchmarks must not compare old truncating semantics to new exact-first
semantics and call the difference an algorithm win/loss. Retain a same-policy
optimization baseline and separately report the new default's real query cost.

Source acceptance is not established by either donor's commit message. Main's
new geo rational/numerical/scratch consumers make replacing whole old files
unsafe. Carry coherent API slices through compiler feedback; preserve owner
and first-error semantics before broad qualification. Independent plan review
is recorded separately; no implementation completion is claimed here.
