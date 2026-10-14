Closes #470

Public integer and decimal values now use the exact numeric tower. Ordinary
division preserves every digit of a terminating quotient and otherwise rounds
to 18 fractional places, half to even. Explicit exact, scale and rounding policies
remain available. Temporal seconds retain their full fractional value.

Rust, CLI, Python, C and synchronous/asynchronous WASM use the same policy.
Governor profile v18 pins the revised answers, fuel boundaries and traces.
Breaking guidance and the English/Chinese book describe the 4.0 migration.
Calendar constructors preserve the requested datatype, including dateTimeStamp;
temporal schema projections agree with exact fractions and midnight normalization.

Current main is integrated. All six original failed targets have passing focused
corrections, including exact 21-place temporal bounds and constructor datatype,
the ordinary AST/CDT replays, and both query/update trace replays. Strict affected
checks and the unchanged helper gate pass. The refreshed optimized package passes
458 tests plus TypeScript, packed installation and full-width identity controls;
42 current installed Python controls pass. The revised ten trace cases preserve
complete answers, intermediate cells and scratch bytes while recording actual
admitted fuel and interruption points.

The repaired required `make check` passed on `0e9a3d9c4`, including all publishable
release WASM libraries. Follow-up `bd4d219bf` removes the two obsolete public AVG
helpers, corrects public rustdoc and current Clippy findings, and names synthetic
allocation counters accurately for CodeQL. Affected strict/rustdoc checks, the
original peak/refusal/release control, metadata/license generation and normal
hooks pass. Existing full/package evidence applies to unchanged computation.
Existing independent numeric, temporal and admission reviews retain their scope;
the follow-up assessment is recorded separately. Current hosted acceptance,
the complete seven-configuration document projection and final integration remain
required.

The public Rust API change belongs to the documented 4.0 migration. No registry
release or numeric-kernel optimization claim is part of this delivery.
