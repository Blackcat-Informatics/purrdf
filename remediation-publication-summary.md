Published remediation in a6fe69de7d10c8c26b1b5425f7bfb26c6141f4bd.

The four inline findings are corrected in the original implementations: both
decision drivers latch cancellation after role reads and before final publication;
top and inverse-top membership are checked before role-order lookup; proof reads
cache the original top role once; and Tableau preserves the original typed storage
refusal before interpreting consistency. Public refusals now retain original term
spelling with the typed classification, including after the private interner dies.
The reproducer also exposed and corrected top-object ABox inventory handling.

The entire affected entail/validate native suite, strict all-target Clippy,
metadata regeneration and helper hygiene pass. Final native and optimized wasm
regular-role controls pass 16/16, including mid-round cancellation sweeps, healthy
neighbors, top outside the RBox and retained source diagnostics. Required commit
hooks passed normally. Independent complete-diff review found no further defect.

Public diagnostic contracts are documented. The bot's 70.35/80 documentation
percentage warning is not a repository gate; blanket comments on private/test
functions would not improve the public contract and are declined.

Current-head CI is running at 38074965475. Original failed assembly jobs remain
recorded as failures. Their seven complete hosted measurement columns were used
to repair the projection, with compiler and source identity retained; those old
reports do not qualify this changed head. Current-head assembly qualification,
remaining CI/review assessment and protected integration remain open. The issue
is not yet complete.
