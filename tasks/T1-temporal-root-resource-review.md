<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Temporal resource integration review

Reviewed 2026-10-12 04:24 MDT in the dirty numeric checkout at HEAD
`2a1a15dcef2b97d87f0af29a208253db0b7df596`. Source is changing during implementation;
this is a findings record, not final-source qualification. Verdict: incomplete.

## Established checkpoint

Read `T1-temporal-governed-admission-receipt.md`. Its later terminal-0 governed
control includes mixed D/H/M with 5,000-digit integral seconds, integral trailing
zeros, negative seconds and malformed-wide neighbors. Root also read the current
shared `exact::cost::decimal_coefficient_digits` home and fallible duration-reader
operations. These address the earlier coefficient-count and infallible-reader
findings in the tested checkpoint. Subsequent arithmetic and recursive-product
edits are explicitly untested; the receipt is not a current full-source pass.

## Remaining production paths

- `value::CanonicalNonNumeric` promises no scratch, but temporal Display performs
  fractional decomposition, integer division and decimal-group conversion. Actual
  evaluator consumers in `expr.rs` calendar casts and `xsd_workspace_value` admit
  only final literal output. Prepared temporal text must retain admitted scratch
  through final-output construction using existing numeric text homes.
- Calendar casts in `expr.rs` drop the parsed owner in an `and_then` closure,
  while `temporal::cast_calendar` identity, DateTimeStamp and DateTime-to-Time
  paths clone exact seconds into an unadmitted raw result. Admit and retain the
  cast result before releasing input ownership. Physical storage errors must not
  become `.ok()` expression refusals.
- Duration subtype casts and seconds component extraction clone exact payloads.
  `adjust_timezone_arg` clones then performs whole/fraction decomposition outside
  admission. Borrow payloads and use existing checked/admitted numeric homes.
- `decimal_add_exact`, `decimal_negate`, `add_seconds_decimal` and `instant_diff`
  used infallible operator bodies in typed-result paths. Their fallible storage
  propagation and original-account ownership require validation after correction.
- `ops::value_cmp_admitted` dispatches temporal families to unadmitted comparison
  helpers, and `value_equal_admitted` invokes duration equality directly. Existing
  evaluator expression and statistical aggregate callers already supply an
  admission callback; temporal dispatch must consume that callback and preserve
  operational refusal independently of incomparability.
- `cost::scale_up_layout` models power construction plus one product destination,
  while `scale_up_using` calls `BigInt::mul_pow10_using` and general multiplication.
  Both operands can cross the recursive threshold. Reconcile the bound with the
  existing recursive multiplication envelope and simultaneous retained owners.
  The temporal alignment envelope needs the same proof. Balanced wide whole
  components and long fractional scales are applicable falsification controls;
  measured controls alone do not prove the symbolic bound.

The numeric owner owns these production corrections and serial evaluator caller
adaptations. The storage owner confirmed no planned overlap in those evaluator
functions. Full temporal, host, portable and workspace acceptance remains open.

## Subsequent source observations

The owner receipt now records XSD test compilation, temporal 82/82 and governed
mixed/wide 1/1 after fallible arithmetic and admitted temporal comparator dispatch.
Root read the new dispatch, which forwards the supplied callback for DateTime,
Date, Time and Duration comparisons and duration equality. A direct comparator
admission/refusal control remains missing. `exact::cost::scale_up_layout` still
has the one-result product bound in the inspected source, independently of the
larger temporal parse envelope; the cost-home finding remains open.

Root also verified a value-space bug against [XSD 1.1 time lexical mapping](https://www.w3.org/TR/xmlschema11-2/#time-lexical-mapping): `24:00:00` and
`00:00:00` map to the same midnight value. Current `read_time` retains hour 24,
and `cmp_time_admitted` uses it as a distinct day-long offset. Normalize the Time
value at its reader home, preserving DateTime's separate next-day semantics.
Verify equality, canonical output and HOURS, including zero-fraction and invalid
nonzero-fraction neighbors. The existing acceptance test for hour 24 does not
prove the midnight identity.

## Correction to the multiplication finding

Root subsequently inspected the complete actual call chain. The earlier
recursive-alignment finding was wrong: `scale_up_using` calls
`BigInt::mul_pow10_using`, which calls `BigInt::mul_using` and the schoolbook
`mag_mul` body. That body allocates one complete `a.len() + b.len()` destination.
`Integer::mul_using` separately selects `BigInt::mul_fast_using`; its recursive
envelope cannot be attributed to the alignment call chain.

Withdraw the proposed recursive-product and carry-word changes to
`scale_up_layout`. The owner has been asked to remove that unnecessary
over-admission. Preserve real fast-kernel bounds at their actual call sites.
Earlier paragraphs retain the mistaken diagnosis for audit purposes; this
correction supersedes them. This does not invalidate the separate reader,
comparator, renderer, cast-ownership or midnight-identity findings.

## Current renderer/cast source checkpoint

Root read `PreparedCalendarText`, its shared prepared decimal groups, and
the actual evaluator `xsd_workspace_value`/`NumericFrame::finish_calendar_text`
caller. DateTime/Time preparation invokes the original numeric render layout
before conversion, retains the render grant with the prepared groups, and keeps
that carrier alive through final literal construction. Calendar and duration
casts copy exact seconds through admitted `try_copy` rather than an unaccounted
clone. These source paths still require current compilation, refusal/peak/release
controls and actual evaluator acceptance. Duration rendering remains outside
the prepared calendar path and unfinished.

Material semantic regression found in new `cast_duration_admitted`: refusing
nonzero months for a dayTimeDuration target or nonzero seconds for a
yearMonthDuration target changes valid casts into refusals. The target retains
only its component; casts between the two specialized subtypes yield zero.
Root verified the specified rule against F&O 3.1 section 19.1.3. The owner
removed those early refusals. Root read the corrected source and new
`duration_subtype_casts_discard_the_other_component` test: mixed `P1Y1DT2S`
casts to `P1Y`/`P1DT2S`, and nonzero cross-subtype casts to `PT0S`/`P0M`.
This is source correction only; its test has not yet run. Wide kept-component
admission/refusal and actual evaluator cast acceptance remain required.

Later checkpoint: root read `T1-evaluator-migration-check.log`, successful
standard Cargo production compile through CDT, geo-kernel and evaluator. The
duration subtype semantic unit, governed kept/dropped duration cast and governed
DateTime/Time prepared-render controls each pass 1/1 in their retained logs.
Root read both governance tests: measured peak stays within the supplied bound,
refused preparation/copy allocates nothing, and the discarded duration component
needs no admission. This supersedes the unrun cast-unit observation above.
Actual evaluator runtime and installed hosts remain unqualified; evaluator
lib-test compilation still has four test-only exact API mismatches.

## Temporal arithmetic callback bypass

Root traced another material resource defect through actual production callers.
`expr::arithmetic_step` creates NumericFrame and calls
`ops::value_binary_admitted`. That function forwards the callback only for numeric
pairs. Temporal pairs still dispatch to unadmitted `value_add`, `value_sub`,
`value_mul` or `value_div_with_policy`. Those paths now allocate exact temporal
payloads and copies. Successful spilled results reach `finish_value` with zero
admitted bytes, and `ParsedValue::from_admitted` refuses their owned heap after
the unadmitted computation. Temporal XsdError::Storage also maps to
NumericOperationError::Value rather than the operational storage channel.

`Function::Adjust` independently calls unadmitted timezone adjustment, whose
retag/shift paths clone exact seconds and perform exact arithmetic; its wildcard
error arms collapse storage failures into `Ok(None)`.

Numeric owner has these corrections. Wire callback-admitted temporal arithmetic
and adjustment through their original homes, preserve retained output grants
and storage causes, and verify actual governed SPARQL wide temporal arithmetic
and ADJUST at admitted/refused budgets. Tiny-value and value-space error behavior
must remain intact. Renderer/cast tests do not qualify these arithmetic callers.

## Exact work charging for temporal values

The same production chain also skips computational admission. Root read
`numeric_cost`/`tower_cost` and `numeric_render_cost`: temporal variants receive
Cost::ZERO even when their exact seconds payload or lexical scale is unbounded.
`arithmetic_step` and `governed_xsd_to_term` use these homes, so wide temporal
arithmetic and rendering bypass the exact-work budget. Duration multiplication
by a wide exact factor is one concrete arithmetic witness. Long fractional
Time/DateTime rendering is an output witness.

The prepared renderer admits native temporary bytes, but that does not discharge
work charging. Extend the existing cost home for actual temporal computations
and underlying seconds/output, retaining the small-value path. Verify governed
exact-work exhaustion before computation/render. The SECONDS expression calls
raw `xsd_to_term` on a cloned Decimal and also needs admitted/borrowed extraction
plus applicable render charging. Owner notified; no correction or qualification
is claimed yet.

## Duration bounded-result spill classification

Root read the later duration add/sub `numeric_cost` source. It charges the
seconds operation only when an input coefficient is already outside i128 or an
input scale exceeds18. Inline duration coefficients can still overflow or
alignment-spill in the exact result: dayTimeDuration i128::MAX seconds plus one
second is a concrete witness. The subsequent generic bounded fallback excludes
temporal variants, so this duration pair currently returns Cost::ZERO despite
requiring the tower. The same condition is already handled for plain numeric
operands. Owner notified to classify actual seconds alignment/add/sub fallback,
retain zero for genuinely machine-word computation, and cover the boundary in
governed work/admission controls. The two-wide-input test does not cover it.

## Duration unary minus admission

Root traced evaluator unary_numeric_step through numeric_unary_admitted. The
fallback Minus arm calls value_unary_minus(Duration), then resident
negate_duration/decimal_negate without the supplied admission callback, and maps
every XsdError into NumericOperationError::Value. Wide seconds therefore allocate
outside the original frame and can reach result finishing with zero admitted
bytes. The owner has the correction at the shared negate home, preserving the
resident wrapper and months-overflow semantics while retaining Storage as an
operational cause. Require a wide actual governed unary control. Calendar
subtraction's negated duration is a live intermediate and must remain charged
through its subsequent admitted computation.

## Instant differences in the complete calendar dispatch

Root traced dateTime/dateTime and time/time subtraction separately from
calendar-plus-duration. Their resident public callers clone exact second fields
into owned Instant values before instant_diff, which creates two raw exact totals
and a raw difference. These pairs also pass through evaluator temporal_sub, so
duration-only admitted arithmetic does not cover them. The numeric owner is
assigned borrowed Instant seconds or admitted copies, timezone-indeterminate
refusal before new allocation, and admission retaining a_total while b_total and
their difference coexist. Actual wide governed dateTime/time subtraction controls
remain required. This is part of the complete calendar arithmetic correction,
not a proposed additional delivery or acceptance reduction.

## Calendar work classification: hour/minute intermediate

Root read the new calendar_arithmetic_cost and actual
add_seconds_decimal_admitted bodies. The first classifier checked source seconds
against duration seconds, omitting the prior hour/minute-base addition. At
12:34:56, an inline i128::MAX duration can therefore spill in the actual total
despite the classifier returning zero. The owner corrected the existing cost
home to check both bounded stages and added that boundary control. Root read the
source correction. The retained calendar checkpoint subsequently records the
boundary control terminal 0, 1/1 passed after rebuilding that cost source. Root
read this result and the scoped independent calendar source review. This resolves
the omission and its leaf regression, not complete calendar cost soundness or
governed-query evidence.
