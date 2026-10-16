# Owner issue assessment

The complete title/body and five acceptance criteria govern this work. Linked
storage indexes do not satisfy statistics or its consumers. The prior-art report
finds no prior delivery for this issue in its searched scope, not proof that no
reusable code exists. Current source already supplies typed access costs,
stats_fingerprint, native ordinal ranges and exact PageSummary marginals.
Retrieval's Statistics trait is a different IRI/ppm contract, not a typed local-ID
predicate-domain provider; duplicating its implementation is unnecessary.

Main risks are duplicate row counts across streams, adding distinct counts,
mixing local IDs across segment dictionaries, inventing exactness from marginal
minima, cache reuse after provider revision changes, and counting statistics
that the actual planner/rule engine never reads. DatasetView and the backend
contract preserve typed identity, first failures and snapshot determinism.
This is owner analysis under the zero-subagent instruction, not independent
approval. Task 1 starts with a bounded common contract; delivery remains the
complete storage/consumer/qualification contract.
