Canonical decimal text now reserves its complete prepared length once instead of growing through generic formatting. Binary64 conversion keeps the inline and range gates together and gives ratio conversion a separate cold frame. Both use the original numerical owners and preserve canonical bytes, rounding, signs and wide values. Broken binary32-width or generated-float-lexical invariants now fail explicitly instead of substituting a number.

Closes #675. This implements the boundary unit under #443; that parent remains open with every original parity, threshold/architecture, allocation/rational and production-query requirement intact.

The benchmark's obsolete bounded/spill labels are corrected: current public value dispatch uses the exact tower, and both fractional-product cases have inline coefficients. Division failures are no longer discarded. Historical toward-zero division is compared only with its matching explicit policy; the current exact-or-18/half-even default is unchanged.

| Native fixed-work fixture | Before instructions/value | After | Change |
|---|---:|---:|---:|
| exact parse/render | 2579.1909 | 2228.2842 | -13.605% |
| public value parse/render | 2593.1870 | 2242.2852 | -13.532% |
| exact binary64 conversion | 149.3594 | 144.3594 | -3.348% |
| public value binary64 conversion | 151.3594 | 146.3594 | -3.303% |

Across 1,024 exact/public parse-render values, allocation calls fall from 1,966 to 1,024, requested bytes from 24,687 to 13,359, and retained/peak bytes from 16,777 to 13,359. The public allocation control includes zero, normalized fractions, `i128::MIN` and 4,096 fractional places with one exact-sized output allocation.

Measurements use acknowledged unscaled instruction windows, ten fresh processes per lane and complete enabled/running coverage. Inputs, warming, output-Vec reservation, full answer validation and destruction are outside the window; typed results remain live. The complete 21-lane matrix and 22 signed wide fixtures against unchanged pre-split source retain all observations and bootstrap intervals. No wide fixture is classified regressed; all 44 paired wide allocation windows agree in all four quantities and retain zero bytes. Keeping range checks with the gate preserves field facts needed by overflow/underflow exits; outlining those checks was rejected on the original growth rows. Measurement scope is native arithmetic sources with returned results retained. Full query and bounded-parity criteria remain under #443.

Validation: 35 native governance cases plus seven frozen/oracle cases; seven direct float-kernel oracle tests; current public native and actual packaged-WASM seven-case replay, including large frozen products/division; the new native/WASM output-allocation case; strict all-target XSD Clippy, complete helper hygiene, formatting, strict rustdoc and exact CI test registration. No shipping dependency, feature, compiler/profile/cache change or byte-golden update. Normal signed commit/push hooks pass. Independent review's product-scale fixture finding is fixed and confirmed; both affected benchmarks execute successfully and doc coverage passes at 85.71%. All 50 hosted checks pass with five configured skips, and the final review thread is resolved.
