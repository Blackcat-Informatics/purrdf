# Numerical recovery review

PASS for the reviewed formatter correction and restored wide-arithmetic tests.
This is not qualification of the complete geo delivery.

Scope: the current working-tree delta in `crates/xsd/src/numeric.rs` and
`crates/xsd/src/wide.rs`, reviewed against HEAD. No builds were run by this
reviewer; only this review record was written.

- The observed failure was an optimizer rewrite: the earlier optimized native
  binary used `ucomisd` / `ucomiss` for the original-bit zero predicate. DAZ
  therefore sent a nonzero subnormal into the explicit zero return. The
  diagnostic log separately showed the native scientific formatter producing
  `5e-324` correctly for the same binary64 input.
- The correction removes that predicate. Native scientific formatting supplies
  signed-zero text, and the existing mantissa/exponent normalization produces
  `0.0E0` / `-0.0E0`. XPath classifies the completed ASCII text and shortens only
  those exact two strings to `0` / `-0`. No bit-comparison spelling, optimization
  barrier, or floating-environment mutation is relied upon in production.
- NaN and signed-infinity branches retain their previous spellings. The XPath
  plain-format interval and its original precision formatter remain unchanged.
  A binary32 subnormal widened under DAZ can still select the scientific branch;
  that branch formats the captured original binary32 value, as the regression
  now exercises. The modified path adds no conversion of its rendered value.
- Public signatures, the 32-byte inline buffer, bounded sink refusal, and the
  owned wrappers' final `to_owned()` allocation are unchanged. The correction
  introduces no heap allocation. Zero now uses the native formatter; its timing
  has not been benchmarked here. Ordinary nonzero finite formatting retains the
  same native formatter and normalization path.
- Restored numeric cases cover signed zeros, finite extremes, subnormals,
  NaN/infinities, every binary64 exponent with representative fractions and both
  signs, full-sink refusal, and changed native flush controls for canonical and
  XPath text. The wide delta changes tests only: all power-of-two divisors use
  a BigInt quotient/remainder oracle, explicit even ties are checked, and 4,096
  full-width cases check the exact nearest-even cell and overflow boundary.

Read verification: `recovery-formatter-and-wide-fixed.log` records an optimized
native XSD library run with **413 passed, 0 failed, 0 ignored**, including the
restored flush-control regression and both wide tests. The root owner reports
terminal exit 0; the reviewed log contains the successful final result.
`git diff --check` for both files also returned 0 during this review.

Portable execution, other hosts, performance acceptance, and the complete final
gate remain unrun for this numerical delta. No unresolved source-level blocker
was found within this bounded review.
