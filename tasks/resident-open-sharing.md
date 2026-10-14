# Existing resident cursor bridge

The existing main resident_property_open! definition is reused verbatim at its
production property_fn home. HNSW and CountingTextRelation now declare their
resident bridge through it; both override open_admitted. S1's fallible typed
capacity API and fixture adaptations remain intact. No main infallible builder
changes were transferred. No new runtime alternative or tooling/ledger change
was introduced.

Affected original all-target Clippy for HNSW and retrieval ran under4GiB,
one Cargo job and default provided tools/directories, with --locked and
-D warnings. Terminal0 in1m02; receipt resident-open-sharing-strict.log.
Independent source review passes in ../reviews/resident-property-open-independent.md.

Unchanged helpers-hygiene53096 is terminal2: original self-tests pass, and
the resident-open duplication group is no longer reported. Four previous groups
still fail: handle pinning, segmented reservation, nesting certificate and
quoted-only capabilities. Whole S1 census and delivery remain incomplete.
The code remains uncommitted; no hook or gate was bypassed.
