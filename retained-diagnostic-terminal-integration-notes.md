# Retained diagnostic integration evidence

The core, terminal and prepared-execution producer units are shipping worktree
source, uncommitted and unqualified. The sole writer applied
retained-diagnostic-core-owner-v2.patch,
retained-diagnostic-terminal-owner-draft.patch and
retained-diagnostic-prepared-producer.patch. These packets passed ordinary
git apply --check and Rust syntax parsing before integration; no new compiler,
allocator or public-query pass is claimed.

The public Query outcome now holds RetainedDiagnostic. Borrowed diagnostic()
still exposes exact code/message/location/presentation; cloning and extracting
the carrier retain its original immutable allocation. Initial control failures
have a separate Copy QueryControlFailure outcome, with stable typed code and
allocation-free formatting. Real allocator refusal remains AllocationFailed.
These additions are non-exhaustive public outcomes; affected consumers need
compilation against this final shape, not assumptions from the previous API.

Native terminal rendering uses the original LexicalFrame/WorkspaceAllocation:
code and message grow through Memory once, then the same grant grows by the
actual Shared control layout before publication. Its ordered stack payload
survives destination refusal and releases text before the grant. Source readiness
is checked after rendering, with a nonsticky cause from the original account
taking precedence. Existing retained diagnostics move through unchanged.

PreparedExecution unbound parameters are now borrowed iteration with one native
Display body, not a collected Vec and joined temporary String. The original
declared order and wording are preserved. Per-run prepared-execution registry
diagnostic text uses the same retained factory. Recognition/registry semantics
are unchanged; registry fingerprint scratch is not certified by this rendering
change.

Actual fixtures integrated, unrun:
- Core retained-layout census including private location/presentation storage,
  heap-free location rendering, and refused code/message growth.
- Initial control outcome formatting/clone with no allocator calls.
- Fatal admitted native callback error through all four direct/prepared/governed
  public entries, shallow clone, receipt-free extraction and last-owner release.
- Prepared execute_fallible unbound-name message with 2KB of names, physical peak,
  retained clone and lifetime after execution/engine destruction.

Remaining complete-delivery work has real owners:
- Writer: full parser/prepared-cache successful and failed physical producers,
  intrinsic authored names/shared controls, plan recheck/rewrite and native boxes.
- Root with writer: remaining raw bare PreparedQuery parameter/registry and
  context diagnostic producers must publish original admitted carriers.
- Numeric helper: full native Tree/Survey/PositivePlan forecasting and EXPLAIN
  constructors/callers, shared reporting owner and operational causes.
- Writer: typed graph/endpoint/term producers remaining after path containers.

The terminal guard for a raw bounded diagnostic explicitly reports unpriced
original production. It is temporary unfinished work, not accepted behavior,
a descope, or a completed error contract. Replace the raw producer route with
its actual admitted producer; do not remove the guard and grant a post-census
certificate. Full acceptance must restore every supported query/error behavior.

Shared and native boxing now have one lex::allocation home, reexported through
core::small. Source review confirms allocation-first factory, original ordered
payload and deallocation-before-grant guarantees. Lex unsafe denial has exactly
two local exceptions for those storage implementations; scanners, algebra and
evaluator callers introduce no unsafe code. Their moved fixtures remain unrun.

Full make check remains 0/3; no PR, commit or merge qualifies this source.
