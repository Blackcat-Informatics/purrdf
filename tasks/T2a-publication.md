Exact-target query access is pushed at `078f9c973` after normal commit hooks.
The shared statement traversal now selects DatasetView's target probe for an
unbound reifier and bound triple. Resident and graph-free carriers use their
actual target orders; Arc preserves those overrides, and other backends retain
the graph-pruned scanning default. Graph order/multiplicity and sticky refusal
are preserved.

The cold/warm traversal allocation control, range/graph parity, all five carrier
tests, six affected BGP controls, strict affected library/test compilation and
native helper check pass. The implementation shares the existing ordinal home;
no dependency, second sort, cache configuration or governor policy changed.

The issue remains open: component and annotation directions, planner/EXPLAIN,
ten-million/six-annotation qualification, portability/conformance and final
delivery are still required. The current validation index states that scope.
