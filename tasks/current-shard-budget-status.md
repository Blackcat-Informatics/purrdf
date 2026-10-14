The final source run cancelled integration shard 5 at its 45-minute job limit
while the selected tests were still running. Cold compilation consumed about
42 minutes of that budget. The cancelled shard is not a pass.

Commit `4070fee39` raises only the test-shard budget to 60 minutes. Compiler,
profile, workers, test selection and assertions are unchanged. Standard
`actionlint` and the existing shard-selection checks pass; normal commit/push
hooks pass. Current CI and Docs are running. The source/package/projection
receipts retain their unchanged scope; final hosted completion remains required.
