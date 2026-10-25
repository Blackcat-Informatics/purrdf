The non-panicking API migration is delivered through #704 / PR #707.
The remaining RDFC native-recursion prerequisite is isolated in #710:
replace n-degree call recursion with owned heap frames while preserving exact
traversal, pruning, successful identity, work charges and stop checkpoints.

Configurable/removable canonicalization budgets, exact configuration/work
evidence, dataset-state controls and heap UDF execution remain in this parent.
The parent stays open. The heap search is a bounded useful delivery, with a
four-hour active engineering/local-validation cap and a two-hour reassessment.
