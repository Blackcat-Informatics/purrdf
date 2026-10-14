# Exact temporal focused runtime checkpoint

Worktree: paudley/470-v4-0-exact-tower-only-and-an-exact-or, dirty source, uncommitted. Applies to source after exact D/H/M numerals, typed temporal storage, retained-byte accessors and inline native 8,193-scale assertion; before governed wide admission and temporal renderer work.

`cargo test --locked -j 8 -p purrdf-xsd --lib temporal::tests -- --test-threads=4`: terminal 0, 82 passed, 0 failed, 336 filtered out.

`cargo test --locked -j 8 -p purrdf-xsd --test exact_governance temporal_native_values_and_deferred_refusals_allocate_no_heap -- --exact`: terminal 0, 1 passed, 0 failed, 14 filtered out. Small temporal values, invalid/refused forms and an 8,193-scale accepted inline Time had zero measured heap traffic.

This is focused leaf evidence, not the full #470 contract. Wide temporal values have no original Memory pre-admission yet. The canonical temporal renderer may allocate ungoverned scratch. Neither issue is closed.
