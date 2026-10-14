# Current numeric regression review

Scope: the two current test-only changes after bbaf0e55f, the original numeric
transcript queries/hand expectations, corrected native and actual wasm32 receipts,
and the existing shared float-environment register implementation.

Source disposition: PASS for these two corrections. This is not complete issue
acceptance or a passing full gate. The original i686 suite is still running.

The distance regression is already gated to x86_64 or x86 with SSE2. The shared
XSD environment::register uses MXCSR for exactly those configurations. Removing
the obsolete x86 Probe expectation keeps the precise typed refusal and exact
register bits assertion on both widths. No refusal is weakened or skipped and
the saved-register/default-neighbor controls remain intact.

The numeric frozen digest predates the exact-default implementation. Root read
the actual current transcript: exact-or-18 retains terminating 1/8 and the tiny
fine/16 quotient, rounds repeating 2/3 to18 places half-even, while explicit Exact
leaves the three nonterminating quotients unbound with FOAR0002. Explicit40-place
half-even and3-place ceiling remain distinct. The unchanged hand expectations
cover promoted integer arithmetic, terminating division, exact binary64-to-decimal,
explicit rounding and division by zero.

The initial native and wasm32 failures retain the same2822-byte transcript and
3704a65139127bbd digest. Corrected native and actual wasm32 targets each pass2/2
with that digest. Only the frozen constant changed; query population, grading,
hand expectations and harness are unchanged. XSD actual wasm32 passes3/3 in its
separate scope. The unchanged full npm check still has three known nesting-floor
failures and is not made passing by these numeric results.
