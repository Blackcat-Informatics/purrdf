# First-head CI ordering failure disposition

Actual first-head a3eb run37865911564 workspace113612474555 failed101 in deny-warning Clippy: items-after-test-module at cost.rs596. Complete raw original failure retained in raw/pr506-first-head/workspace.log; first run remains failed regardless of later correction.

Root corrected only source ordering: moved the existing comparison_bounds_tests cfg(test) module unchanged to the end of cost.rs. No allow, expectation, production operation, cost formula or data layout changes. Root reports normal final tuple commit32110 actual0/44a6d3e93 and normal ordering-fix hooks40552 running. Qualified96826 local/performance receipts apply to unchanged production/numeric source; no new arbitrary broad benchmark/native/host rerun is imposed for the move.

Fresh published final-head CI on actual floating hosted nightly must prove this lint correction and the outstanding host/full-suite contract. This disposition is pending that real head/terminal, not a current green claim. Monitor will update the existing PR body and start the new exact-head watcher after ordinary push succeeds; no duplicate PR or manual rerun is needed.
