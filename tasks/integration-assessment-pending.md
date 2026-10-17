# Preliminary integration assessment

Fetched origin/main is ff302c091, the native graph-projection delivery. The
current BLAKE3 head is 2c9c08dc5. git merge-tree --write-tree --no-messages
origin/main HEAD succeeds with tree bfebc29df50ec8c7a6af2a5d5edcaef1bf8bca68.
This is a clean prediction, not an execution or final merge authorization.

The base delta changes the graph-selection predicate and project_graph API,
its direct canonicalization/pipeline consumers and tests, and their helper
source receipts. The BLAKE3 delta adds mode-aware dispatch/output while retaining
the existing unkeyed APIs and all frozen content digests. No graph caller or
projection signature is changed by this delivery. The helper-ledger edits merge
cleanly and concern separate existing job entries.

No base synchronization or restarted CI is justified just by being behind.
Final current-head CI, projection/document publication and complete feedback
remain pending. Refresh the inputs and reassess their delta before ghprsq.
