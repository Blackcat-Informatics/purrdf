The docstring-percentage warning is declined as a blanket target. The new public
probe/support methods and fields document their graph, ordering, availability
and estimate semantics, and the backend/profile guides are updated. The metric
counts 120 touched functions, including private implementations, forwarding
methods and test cases; it identifies no concrete undocumented public contract.
Hosted rustdoc passes. No checker or threshold was changed.

The completed source review reports no actionable correctness or availability
finding on the current head. The local full gate and remaining hosted checks
must still finish before merge.
