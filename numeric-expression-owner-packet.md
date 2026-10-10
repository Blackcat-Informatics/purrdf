# Numeric expression ownership and checked admission

Source inspection only, in the selected 508 worktree. No source edits, builds, tests, forge actions or ref changes were performed for this packet. This supports the sole implementation writer; it is not another completion audit. The proposals preserve the existing numerical implementations, division laws and lexical spellings. No 443/470 optimization is required.

## Immediate implementation decisions

1. Make operational numeric parsing return a fallible, owner-carrying internal value. The existing source lease covers the borrowed lexical bytes, not the newly decoded magnitude, cache copy, numeric operation or rendered string. Admit each of those actual owners before allocation.
2. Reuse `exact::cost::Shape`, its binary-bit digit bounds and the existing kernel cost functions at their XSD home. Supply checked allocation-layout counterparts there; do not copy their arithmetic into the evaluator or turn `Cost::ZERO` into a no-allocation certificate. Preserve successful inline branches.
3. Correct the two concrete `decimal_div` estimates below. The shape-only exact-division bound is useful for the existing deterministic governor. For tight operational workspace admission, stage the native exact quotient and price the actual final exponent after factoring, immediately before `scale_up`.
4. Admit canonical rendering independently, then move the admitted lexical string into the typed term instead of formatting to a temporary and cloning it. Keep the numeric operation owner alive through rendering. Transfer surviving lexical ownership to scratch rather than dropping its lease while its bytes remain live.
5. Cache immutable parsed values once and borrow/share them without deep cloning their magnitudes. Charge cache table growth, immutable payload/header and any transition to shared limb storage before allocation. Keep the cache owner alive through all internal users; do not expose a raw numerical value that can retain shared magnitude bytes after its admission owner disappears.

## Actual seams and omissions

| Source home | Current behavior | Required admission boundary |
|---|---|---|
| `sparql-eval/src/expr.rs::xsd_of_term` and `eval.rs::EvalCtx::xsd_parse_cache` | Existing-ID misses parse under `with_term`, then `parsed.clone()` into the map; hits return `cached.clone()`. Big magnitude clones can allocate. Computed terms also parse new owned values. | Borrow lexical guards; obtain a checked parse plan before parsing; retain the parsed payload once. Cache entry/table/header admission precedes insertion. Return an internal owner carrier or borrowed/shared view, not a deep copied `XsdValue` by default. |
| `expr.rs::xsd_of`, `eval_xsd_cast`, equality/comparison/EBV callers of `xsd_of_term` | Parsing occurs before `numeric_step_admitted`. `.ok().flatten()` currently collapses datatype parse errors. | Change operational entry to `Result<Option<OwnedParsedValue>, EvalError>`; lexical/type errors retain their current F&O behavior, but workspace/storage errors propagate as the original operational failure. Arithmetic-only gating cannot cover these parses. |
| `xsd/src/numeric.rs::parse_decimal` | Allocates `format!("{int_str}{frac_str}")` even when the result fits `i128`; an out-of-range error clones the original lexical before `value.rs` retries on the exact tower. | Avoid those allocations in the shared parser: classify bounded overflow without an owned error, accumulate bounded digits directly, then use the exact parser. If an owned diagnostic is required, admit its actual lexical copy only on the error branch before building it. |
| `xsd/src/exact/decimal.rs::FromStr` | For more than 38 coefficient digits builds a concatenated signed coefficient `String`; `Decimal::new` may copy/normalize a big coefficient. | Reuse `BigInt::from_decimal_bytes(whole.bytes().chain(fraction.bytes()))` after the same lexical validation, rather than constructing concatenated text. Handle sign without an unnecessary magnitude clone. Price actual normalization or canonicalize the borrowed digit slices before constructing the same validated value. |
| `xsd/src/bigint.rs::mag_from_digit_bytes` | Decimal chunks of 19 digits repeatedly call `mag_mul_small_add` with `Unbounded`; old and new magnitudes coexist. | Provide an admitted native parse entry using the existing storage seam or a checked parse-frame layout that covers the actual two live destinations and their requested capacities. No evaluator-side digit parser. |
| `xsd/src/numeric/exact_path.rs` | `integer_of`/`decimal_of` clone exact operands before add, multiply, divide and some comparisons. | Shared immutable magnitudes make those clones shallow; otherwise admit the newly copied magnitude bytes separately from the original operand. Cost estimates alone do not price promotion copies. |
| `expr.rs::arithmetic_step`, `unary_numeric_term`, numeric casts and aggregate folds | Current exact governor charges work/scratch, then performs native arithmetic. Bounded overflows recompute on the exact tower even when `numeric_cost` returned zero. | Select the actual inline/fallback branch with the existing checked bounded arithmetic; charge the fallback kernel and promotions before entering it. Workspace admission is additional to the existing deterministic governor charge. |
| `expr.rs::xsd_to_term`, `xsd_literal_value`, `xpath_value_to_string` | Produce a canonical `String`, then `typed`/`typed_term` copy its lexical and datatype. | Admit render scratch plus destination; move the owned lexical into the existing typed-term constructor/interner. Preserve XPath string-cast rendering separately from XSD canonical literal rendering. |
| `modifier.rs::aggregate_numeric_cost` and `exact/cost.rs::compare_chain` | MIN/MAX builds a `Vec<Shape>` before the returned cost is charged; `compare_chain` builds another keyed vector. | Charge both exact vector capacities before calling the estimator, or reuse admitted supplied scratch/stream a running-extreme estimator at the same cost home. A function computing a bound must not allocate outside admission. |
| `exact/rational.rs::to_decimal_cost` | Clones numerator and denominator merely to obtain a decimal division estimate. | Read shapes without cloning; cost calculation itself must remain allocation-free or have a prior owner. Rational is an existing exact API/custom-native boundary, not an additional built-in SPARQL datatype. |

## Checked quantities and composition

Use actual requested capacities, with `checked_add`, `checked_mul`, `checked_sub`, `checked_div`/ceil division and checked conversion to `usize` and `u64`. A size overflow is an actionable operational admission failure before allocation. Existing saturating `Cost` is the deterministic governor representation; it cannot certify that a physical layout was computed without overflow.

Definitions, using the existing native home:

* `L(d) = ceil(d * 3322 / 64000)` is the certified binary-limb upper bound for `d` significant decimal coefficient digits (`exact::cost::limbs_for_digits`). Its intermediate multiplication fits `u128` for every `u64` input. Keep it at that home.
* `D(bits) = ceil(bits * 301029995663981196 / 10^18)` (and one digit for zero) is the certified decimal digit bound from binary bit length. `Integer::shape` already reads limbs and bit length without decimal conversion. Do not use `Integer::decimal_digits`/`BigInt::decimal_digits` to price rendering: the compatibility accessor renders a string.
* `word_bytes(c) = 8*c`, where `c` is the requested/retained capacity, not just the magnitude length. Actual three-limb inline magnitudes need no heap allocation. `Mag::with_capacity` uses inline storage when the requested capacity is at most four, but a fourth pushed limb spills into a four-word heap vector; preserve this actual distinction.
* Original owned operands are already admitted. A new detached/deep clone is a new allocation, even if its numerical value is unchanged. A shared immutable clone has zero new limb payload, but an initial `Arc`/owner header must be admitted. `BigInt::shared_owned_bytes` and `allocated_bytes` describe the existing representation; account checked versions of their layout calculations.
* Overlapping owners add: `live = admitted_source + admitted_cache + new_promotions + kernel_temporary_and_result + render_temporary_and_payload + surviving_term_metadata`. Sequential scratch can use a maximum only after the preceding scratch is actually released. `Cost::then` explicitly assumes that release; it is not the composition rule for a live operand, result or cache.
* Do not charge an already admitted retained payload twice merely because it has two views. Keep a carrier/lease with its immutable payload, or transfer the lease with the payload. New physical copies are charged separately. Arc count changes alone are not payload copies.

### Parsing and decimal radix conversion

The actual integer and decimal lexical parsers accept decimal digits with their specified sign/decimal-point rules; there is no built-in arbitrary-radix integer or decimal exponent syntax here. Decimal text is converted to base-2^64 limbs by 19-digit chunks. Rendering converts back through base-10^19 groups. Floating-point lexical exponent syntax is a different branch and must not be mistaken for an instruction to build `10^exponent` as a big integer.

For a validated integer lexical, scan/trim leading zeros without allocation. Let `d` be the remaining digit count, zero when the value is zero. Each `mag_mul_small_add` asks for `a.len()+1` words, while the preceding magnitude is still live. For a non-inline parse, a checked `16*(L(d)+1)` parse-frame bound covers those two destination capacities; the existing `cost::parse(len).bytes() = 16*L(len)` omits this extra requested-capacity word and all lexical/error buffers. Use a zero-heap plan for an actual inline parse, not that non-inline bound unconditionally. Preserve sign and digit validation for all original bytes.

For decimal, the original fraction length must first fit the existing `u32` scale law. Normalized shape is not automatically parse storage: `Shape::of_lexical` trims leading and fractional trailing zeros, whereas today's parser can concatenate all those bytes before normalizing. The economical repair is to normalize the borrowed coefficient slice/iterator and scale before native construction, using the same exact lexical/value law. Then run the admitted decimal-byte iterator parser and avoid the coefficient string entirely. If the existing `Decimal::new` normalization remains, admit its clones, trailing-zero scan/division working set and possible power-of-ten division before that call; the two-parse-destination frame alone does not cover it.

Invalid input can currently allocate `XsdError::invalid`/`ExactError::invalid` lexical copies. Keep allocation-free lexical classification separate from materializing an owned public error. Operational callers that only need the F&O code need not copy the offending lexical. If a diagnostic retains it, admit `lexical.len()` plus actual string/record/header layout before construction. Do not swallow an allocation refusal as an invalid lexical.

`parse_ieee`/`parse_float` return machine values through Rust's float parser and check XSD spellings. Exponent length does not grow the output value. Preserve INF/NaN handling and the existing XSD 1.0 `+INF` refusal in casts. Their explicit error branches can copy arbitrary-length offending lexicals and still need the preceding diagnostic boundary.

### Operations: use native branch plans, not a stored-term multiplier

| Operation | Existing useful native bound | Additional owner obligations |
|---|---|---|
| Integer add/sub | `cost::add(La,Lb)`, result capacity at most `max(La,Lb)+1` | Actual exact operand promotions/clones; inline success has no limb allocation. |
| Integer multiply | `cost::mul(La,Lb)`, result limbs at most `La+Lb`; current home distinguishes schoolbook/Karatsuba working sets | Keep its established kernel scratch law; do not invent an evaluator multiplier. Original operands stay live. Account any exact-path clone before multiplication. |
| Integer division/remainder and gcd | `cost::div`, `cost::gcd`, existing `_in` implementations | Divisor/numerator promotion copies and result/remainder ownership coexist with the old operands. A `Cost::then` maximum is valid only when intermediate results are released. |
| Decimal add/sub | `cost::decimal_add`, gap `abs(sa-sb)` scales the lower-scale coefficient | Add promotion copies; scaling the selected coefficient, aligned operands and sum are simultaneous owners. If sign/zero shortcuts decide without scaling, preserve them. |
| Decimal multiply | `cost::decimal_mul`; coefficient limb bound `La+Lb`, checked scale `sa+sb` | Preserve actual normalization ownership. A small coefficient can have a large result scale and therefore a large later lexical payload. |
| Decimal rounded division | `shift = requested_scale + sb - sa`; `digits = abs(shift)`; scale A for nonnegative shift and B otherwise; reuse `shift10` + `div` | Select the shifted operand by the sign, not its length. Inline `div_rounded_small` success allocates no limbs. Before the general branch, admit abs/promotion copies, scaled coefficient, quotient/rounding and normalization. |
| Decimal exact division | Native gcd, reductions, factor stripping, `k=max(twos,fives)`, coefficient multiplication, final exponent `sb-sa-k` | The final positive power of ten is a real extra owner currently absent from cost. Do not perform that `scale_up` before admitting it. See correction/diff below. Nonterminating division retains its existing domain error. |
| Unary/round/precision functions | `decimal_unary`, `round_cost`, existing early sign/zero/precision branches | The operand clone is separate; negative requested precision can require a later power-of-ten scale-up. Admit before the actual branch; preserve shortcuts that do not construct an enormous power. |
| Exact/IEEE promotion and comparison | `decimal_to_float`, `decimal_cmp_f64`, `Shape::ieee_cost`; sign/binade separation can decide cheaply | Do not force a giant decimal denominator for a value whose certified magnitude already determines zero, infinity or order. Float-to-exact conversion uses the actual IEEE significand/exponent and native factor/shift bounds; `Cost::ZERO` for a machine source is not proof its exact destination has no heap. |
| Pow/power-of-ten | Native `pow`, `shift10`, and `BigInt::*_in` scratch path | `pow_cost` result-only bytes are not automatically a bound for simultaneously live base/result/squaring products. Use native repeated-squaring destinations/staged admission, especially before each growing multiply. Do not expand a lexical IEEE exponent through this path. |
| Rational conversion/arithmetic (existing exact/custom boundary) | Existing add/mul/reduce/to-float cost homes | Numerator and denominator clones, reduction inputs/results, decimal power-of-ten denominator and both rendered components are distinct owners. Preserve division by zero/reduction semantics. |

The XSD foundation cannot depend on `purrdf-core` or `sparql-eval`. Keep checked allocation layouts and any narrow admitted-kernel callback/trait in XSD using primitives and its existing `bigint::Allocate`/`LimbScratch` seam; evaluator supplies the workspace owner. `LimbScratch::required_bytes(buffers,limbs)` already checks each word buffer, free-list slot, `Arc<Vec<u64>>` header and arena owner before construction. Its admitted `_in` operations refuse exhausted destinations/capacity without allocating. Reuse it for a genuinely fixed operation plan; do not allocate a per-query largest-input arena or assume every exact API already routes through it (parsing and Display currently use `Unbounded`).

For the common path, a checked layout at the cost home plus one scoped workspace frame before the native call is sufficient, provided it includes the actual promotion copies and kernel working set. Where a useful bound is available only after a native phase (exact division's factor stripping, arbitrary exponentiation), expose a narrow before-allocation continuation at that native seam. This continues the same algorithm and semantics. It does not justify refusing a supported native extension, changing an arithmetic policy or using a blanket worst-case query allocation.

### The two concrete division corrections

Rounded: replace `shifted=min(la,lb)` with `shifted = la if shift>=0 else lb`. This follows exactly `Decimal::div_rounded`; current `numerator`/`denominator` bounds already use the correct sign.

Exact: the existing factor multiplier bounds the pre-shift coefficient by `C = la + 3*lb` limbs. Since `k>=0`, the positive final decimal exponent is at most `gap=max(0,sb-sa)`. Add `shift10(C,gap)` to the existing exact cost when `gap>0`. This is a certified shape-only bound for the missed operation. For operational workspace admission, use the actual post-factor coefficient and `e=max(0,sb-sa-k)` immediately before the final `scale_up` when that gives a materially tighter bound. Avoid treating the full gap bound as an obligatory large allocation for healthy calls whose factor count cancels it.

Concrete native counterexample: `Decimal::ONE / Decimal::new(Integer::ONE, 10_000)` under `Exact` is the integer `10^10000`. Original coefficients each have one limb. The old Exact cost is independent of the scale gap; the result is thousands of bytes and exceeds that cost. Both `result.heap_bytes() <= cost.bytes()` and operational refusal before the result's scale-up are meaningful checks. With enough admitted storage, the exact same quotient and lexical bytes must succeed.

The proposed diff below changes only the cost defect and adds focused native regressions. It uses the existing saturated governor representation; the separate checked physical-layout boundary described above is still required. It is a packet for the source writer to apply/review, not an applied change or evidence that any test passed.

The rounded regressions execute asymmetric native divisions with independent decimal answers and use the existing `exact_governance` counting allocator/work-traffic hooks. They do not reconstruct the cost formula. That existing harness permits `4*cost + 4096` peak bytes, so its pass alone cannot certify exact physical workspace admission; the operational tests must separately prove refusal before allocation and live/peak bytes within the actual admitted owner. The exact positive-gap retained-result assertion directly exposes the old missing cost.

```diff
diff --git a/crates/xsd/src/exact/cost.rs b/crates/xsd/src/exact/cost.rs
--- a/crates/xsd/src/exact/cost.rs
+++ b/crates/xsd/src/exact/cost.rs
@@ -772,16 +772,28 @@
             } else {
                 (la, lb.saturating_add(limbs_for_digits(digits)))
             };
-            let shifted = if la < lb { la } else { lb };
-            shift10(shifted, digits).saturating_add(div(numerator, denominator))
-        }
-        super::DivisionPolicy::Exact => {
-            let strip = lb.saturating_mul(128).saturating_mul(lb.saturating_add(1));
-            gcd(la, lb)
-                .saturating_add(div(la, 1))
-                .saturating_add(div(lb, 1))
-                .saturating_add(Cost::new(strip, 0))
-                .saturating_add(mul(la, lb.saturating_mul(3)))
+            let shifted = if shift >= 0 { la } else { lb };
+            shift10(shifted, digits).saturating_add(div(numerator, denominator))
+        }
+        super::DivisionPolicy::Exact => {
+            let strip = lb.saturating_mul(128).saturating_mul(lb.saturating_add(1));
+            let reduction = gcd(la, lb)
+                .saturating_add(div(la, 1))
+                .saturating_add(div(lb, 1))
+                .saturating_add(Cost::new(strip, 0))
+                .saturating_add(mul(la, lb.saturating_mul(3)));
+            // div_exact finishes coefficient * 10^(sb - sa - k) when the
+            // exponent is positive. k >= 0, and the coefficient before that
+            // scale-up occupies at most la + 3*lb limbs.
+            let gap = b.scale.saturating_sub(a.scale);
+            if gap == 0 {
+                reduction
+            } else {
+                reduction.saturating_add(shift10(
+                    la.saturating_add(lb.saturating_mul(3)),
+                    gap,
+                ))
+            }
         }
     }
 }
diff --git a/crates/xsd/tests/exact_governance.rs b/crates/xsd/tests/exact_governance.rs
--- a/crates/xsd/tests/exact_governance.rs
+++ b/crates/xsd/tests/exact_governance.rs
@@ -219,3 +219,58 @@
     assert_eq!(double, 1e308);
     assert_bounded("10^308 to f64", cost, measurement);
 }
+
+/// A final exact scale-up is part of the quotient's actual retained storage.
+#[test]
+fn exact_division_positive_scale_gap_covers_retained_result() {
+    let divisor = Decimal::new(Integer::ONE, 10_000);
+    let cost = Decimal::ONE.div_cost(&divisor, DivisionPolicy::Exact);
+    let expected = format!("1{}", "0".repeat(10_000));
+    let (result, measurement) = measured(|| {
+        Decimal::ONE
+            .div(&divisor, DivisionPolicy::Exact)
+            .expect("a power-of-ten divisor terminates")
+    });
+    assert_eq!(result.scale(), 0);
+    assert!(result.heap_bytes() <= cost.bytes(), "{cost:?}");
+    assert_eq!(result.canonical_lexical(), expected);
+    assert_bounded("exact positive scale gap", cost, measurement);
+}
+
+/// (3*10^2000 + 1)/3 = 10^2000 + 1/3, rounded to two places.
+#[test]
+fn rounded_division_prices_a_long_scaled_dividend() {
+    let coefficient: Integer = format!("3{}1", "0".repeat(1_999))
+        .parse()
+        .expect("an integer");
+    let dividend = Decimal::from_integer(coefficient);
+    let divisor = Decimal::from_integer(Integer::from(3));
+    let policy = DivisionPolicy::scale(2, Rounding::HalfEven);
+    let cost = dividend.div_cost(&divisor, policy);
+    let expected = format!("1{}.33", "0".repeat(2_000));
+    let (result, measurement) = measured(|| {
+        dividend.div(&divisor, policy).expect("a rounded quotient")
+    });
+    assert_eq!(result.canonical_lexical(), expected);
+    assert!(result.heap_bytes() <= cost.bytes(), "{cost:?}");
+    // This measures the real native computation, including its temporary
+    // coefficients, rather than reconstructing the estimate's formula.
+    assert_bounded("long rounded dividend", cost, measurement);
+}
+
+/// 0.01/(3*10^2000 + 1) rounds to zero; its denominator still grows by 10^2.
+#[test]
+fn rounded_division_prices_a_long_scaled_divisor() {
+    let coefficient: Integer = format!("3{}1", "0".repeat(1_999))
+        .parse()
+        .expect("an integer");
+    let dividend = Decimal::new(Integer::ONE, 2);
+    let divisor = Decimal::from_integer(coefficient);
+    let policy = DivisionPolicy::scale(0, Rounding::HalfEven);
+    let cost = dividend.div_cost(&divisor, policy);
+    let (result, measurement) = measured(|| {
+        dividend.div(&divisor, policy).expect("a rounded quotient")
+    });
+    assert_eq!(result.canonical_lexical(), "0");
+    assert_bounded("long rounded divisor", cost, measurement);
+}
```

## Canonical numeric payload and transient rendering

For an integer coefficient of binary length `bits`, take `d=D(bits)`, at least one. For exact decimal scale `s`, a safe canonical text bound is `sign + d` when `s=0`, `sign + d + 1` when `0<s<d`, and `sign + s + 2` when `s>=d` (`0.` plus required fractional positions). Include the possible sign with the actual sign or a one-byte bound. The bounded decimal variant keeps its existing `.0` spelling for an integral decimal, so do not substitute the exact decimal formatter's scale-zero text law blindly.

The native render bound already accounts for coefficient conversion: with `g=ceil(d/19)` and `t=rendered_text_bound`, `render_parts` uses `8*(3*L+3*g) + 6*t`. These coefficients come from the existing quotient/group/string owners, including old/new coexistence and geometric string capacity. Implement the same expression with checked arithmetic at the XSD home. Do not copy an arbitrary multiplier into evaluation. A tighter supplied destination must reserve its certified length before writing; any formatting error/needed extension must be handled before extending it.

`BigInt::Display` holds a coefficient clone, successively allocated quotients and a growing base-10^19 group buffer; `Decimal::Display` additionally obtains `abs().to_string()` and writes the padded fractional text. A counting `fmt::Write` pass over an arbitrary big number also performs that conversion and can allocate; it is not a free length calculation. Read the binary-bit `Shape` bound instead, or use an admitted native renderer with the existing limb storage seam.

Small integer/decimal output is still newly allocated text despite `numeric_render_cost` returning zero. `i128` text needs at most 39 magnitude digits plus sign. The actual bounded decimal `mantissa`/scale determine its sign, dot and zeros without heap arithmetic. Price that output string capacity before `to_string`/canonical rendering, and move it into the result. Existing admitted lexical operands do not pay again merely because their values are rendered; the new rendered payload does.

Floating rendering has explicit transient owners: `canonical_double`/`canonical_float` create `raw=format!("{value:e}")`, then an output buffer of `mantissa.len()+exp.len()+3`; both coexist. Special INF/NaN/signed-zero strings allocate too. The IEEE significand/exponent widths are finite, so reuse the standard primitive formatter in a fixed-stack or already-admitted destination/counting sink, then price the exact normalization destination before writing. A scalar counting-format pass has no big-integer conversion; preserve the resulting shortest-round-trip digits. For XPath numeric-to-string preserve its fixed/scientific threshold and spellings (`xpath_float_to_string`/`xpath_double_to_string`) rather than always invoking XSD canonical scientific formatting.

At `xsd_to_term` prefer an owned lexical constructor (`TermValue`'s existing literal shape, fed by the moved `String`) and the existing datatype IRI home. Admit the datatype copy if that representation owns it. If a temporary canonical string and a copied final string remain, both capacities must be admitted while they coexist. Final scratch/result accounting must retain the lexical payload after the render frame is dropped; it must not merely recount the bytes after their allocation.

## Aggregates and opaque numerical extensions

Built-in SUM/AVG keep original survivors and a running total live. Reuse `SumChain`'s mathematical bound: for at most `n` coefficients with at most `w` whole digits, the total has at most `w+ceil(log10(n))` whole digits at the maximum retained scale, not one extra digit per row. Charge each new parsed operand, total/promotion and final render before allocation, retaining survivor owners throughout. A bounded running sum can overflow to exact while the old aggregate cost fast path says zero; its actual inline/fallback operation still needs admission. AVG additionally prices the chosen division policy and quotient render. MIN/MAX parse/compare ownership and the estimator vectors are separate from survivor storage. GROUP_CONCAT/string output stays with the string owner packet.

`eval_custom_aggregate` currently has governance declarations `state_bound` and `exact_numeric_cost_under`; those are not automatically live workspace leases. Admit declared accumulator state plus actual box/header before `init_under`, and extra accumulators before any permitted chunking (the operational path is sequential). Cover both state-owned numeric buffers and fold-temporary numeric work according to the declared contract; avoid double charging a result already covered inside that declared state. Scalar-value names/terms, argument tuples, DISTINCT copies and survivor vectors have their own real capacities. `finish` returning an already-created arbitrary `TermValue` cannot be made before-allocation-safe by pricing only at `try_intern_checked_admitted` afterward. A narrow bounded finish/output declaration or admitted construction context must cover result creation and the overlap while consumed state is still alive; preserve all currently supported aggregate semantics. The native extension's cost/declaration method itself must not perform unadmitted allocation. No silent no-op or refusal of an otherwise supported configured aggregate replaces this boundary.

Rational remains an exact/custom consumer: parse two admitted coefficients, admit sign/reduction temporaries, retain numerator+denominator ownership, then price each component conversion plus the final slash string. Its decimal conversion can create `10^scale` and reduced fractions; its IEEE conversion uses actual finite significand/exponent bounds. Reuse the existing native operations and cost home.

## Focused executable boundary cases for the writer

These are acceptance suggestions for the existing one coherent delivery, not additional gates run here:

* Repeated cache misses/hits on a large numeric literal: first parse/table growth is admitted before allocation; later hits do not duplicate magnitude payload. Small/zero/leading-zero integer and decimal lexicals keep their inline behavior. Dropping a temporary cache view cannot release storage still used by another view.
* Long fraction/leading zeros/trailing zeros: correct value and normalized scale, no concatenated coefficient string or unpriced diagnostic clone. Invalid huge lexical still produces the existing F&O behavior; workspace refusal remains an operational error.
* Bounded `i128` addition/multiplication overflow and a SUM promoted beyond machine words: exact parity with resident evaluation, tiny operational budget refused before tower allocation, sufficient budget succeeds. Default/nondefault rounded and exact division are all exercised.
* The `1 / (1*10^-10000)` exact quotient above: native result bytes fit corrected cost; operational tiny budget is refused at or before scale-up; enough budget succeeds with the same result. Test rounded shifts of each sign with the longer operand on the shifted side. Include zero and nonterminating `1/3` so numerical errors remain distinct from workspace failures.
* Very large scale with tiny coefficient: rendering is priced as its real zeros/output, independently of stored-term size. Negative precision and IEEE-to-exact branches keep their actual fixed/input-sensitive bounds and early exits.
* IEEE tiny/subnormal/max-finite/signed-zero/INF/NaN canonical and XPath string casts: old lexical bytes remain identical; both raw and normalized render buffers, if retained, are admitted.
* Custom aggregate state and finish returning a long numeric lexical: refusal occurs before state/result allocation; sufficient budget succeeds, unchanged F&O/language-string behavior. Surviving result clones/extraction retain the admitted payload ownership according to the retained-result API.

No test outcome is claimed. Remaining concrete owners are the parse/cache lifecycle, promotion/kernel fallback, the two corrected division growth branches, canonical/XPath rendering and custom aggregate finish. Closing only the arithmetic governor check leaves these operational allocations unbounded.
