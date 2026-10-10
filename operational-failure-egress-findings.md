# Operational refusal ownership findings

Current source observations; these are required fixes, not acceptance results.

Source update: account growth/retention/resize now returns WorkspaceStopped
without rendering or cloning E. `take_failure` moves E while `failed` remains
latched; Drop cannot replace the first cause. This addresses findings 1 and 2
inside the account. Shared deferred evaluation egress is still being wired;
runtime and allocation acceptance below remains unrun.

1. `workspace.rs` calls `EvalError::source_read` after reservation refusal.
   That helper allocates a secondary String. Carry an allocation-free internal
   operational-stop error instead; the account retains the original typed cause.
2. `QueryWorkspace::failure()` and growth paths clone generic `E` on refusal.
   `Clone` does not imply allocation-free. Use a latched terminal failure flag
   for internal control flow and move the first owned cause into the final typed
   receipt. Taking the cause must never permit a failed account to reopen.
3. `finish_fallible_query` checks the account before publishing, but receives an
   already formatted `RdfDiagnostic`. Check operational failure before earlier
   diagnostic conversion too; allocating then discarding the diagnostic is not
   before-allocation admission. Preserve final provider failure precedence.
4. Exact decimal division's existing cost omits the final positive scale-up.
   Coefficient 1 at scale 0 divided by coefficient 1 at scale 10000 produces
   10^10000, although the old cost depends only on the one-limb coefficients.
   Correct the existing cost home, without changing division semantics. Rounded
   division also chooses its shifted operand by shift sign, not min limb count.

Acceptance: non-latching provider refusal retains its exact first typed cause;
no diagnostic/Clone allocation follows refusal; governor cancellation and query
errors cannot replace it; repeated account access remains failed. Exact numeric
cost covers the actual produced heap for asymmetric scale/limb cases, healthy
bounded/resident results agree, and a smaller real budget refuses before numeric
allocation. These checks have not run.
