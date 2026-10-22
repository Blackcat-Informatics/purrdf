The stale seal finding is fixed in 55e3b0527. Successful row removal now discards
the retained preparation and forwards its admitted bytes through the existing
retirement accounting. Absent removal preserves valid retained work.

The public regression first reproduced the exact 64 versus 63 pinned-shape panic.
It now passes on native and the shipped WASM runner. A supplemental control also
covers zero/partial/complete preparation, sealed-batch withdrawal, exact retired
frame bytes and resumed insertion without resurrecting the removed row.
Current qualification passes 467 Datalog library cases, 49 public runtime cases,
all 5 allocation cases, 49 WASM runtime plus 16 WASM public refutation cases,
affected all-target strict checks and shared-helper/hash-domain gates.

The random-program acceptance control is
`maintained_transactions_match_seeded_random_signed_histories` in
`crates/datalog/tests/rules_runtime.rs`. It generates 8 seeded recursive programs,
each with 8 random recursive rule choices, two negation strata and 96 signed
transactions. Every completed transaction compares full facts, exact net delta
and canonical chosen derivations with cold evaluation, then independently checks
grounding. It is registered and executed in both current 49-case runtime runs.

Five native assembly jobs reported only measured-cell drift in `datalog.chase`.
The documented cells now reflect their actual observations through the existing
writer. Updated-head hosted checks remain required; no CI exception is requested.
