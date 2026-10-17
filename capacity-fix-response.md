The workspace CI failure was stale source receipts in the logical-capacity
ledger after the graph API and its tests changed the pinned kernel files.
Commit `af8936e6f22c70dcbc3f3dc1224be887aba51cc5` renews the dataset authority
identity, constructor-closure identity and the moved permutation sink address.
No proof class, sink identity, reason, authority set, limit or checker was changed.

The complete `make helpers-hygiene` now passes: all 82 enforced ledger jobs,
118 distinct-body rows and all 133 registered hash domains. The initial local
attempt correctly failed because it used the function declaration's line
instead of the observed sink line; that was corrected before qualification.
The original 1459 passing kernel and consumer tests remain applicable because
this repair changes only ledger source receipts. Normal signed commit hooks
and the normal push succeeded. Hosted checks are refreshing on the new head.

The linked-issue review warning about reviewable downstream integration is still
being addressed. The GMEOW worktree deletes the authored-default shortcut,
projects every selected world from the complete object EDB and migrates its
named callers. Its consumer type-check passed against the new native API.
The dependency pin now includes this receipt repair; current strict Clippy and
the explicit optimized default-world benchmark are running before publication.
I will add the actual dependent commit/PR and execution result here; no full
canonical admission or complete engine delegation is claimed.
