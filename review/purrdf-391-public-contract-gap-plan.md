## Public contract documentation remediation

The independent audit prompted by the aggregate documentation advisory found concrete stale public contracts, although runtime/carrier controls establish the implemented behavior:

- MEDIUM: `EvalCtx::with_bnode_mint_prefix` and the public prefix field promise that a prefixed suffix equals the unprefixed mint's suffix. Vacancy is checked on the complete label, so a dataset containing `tag_bnode1` may make the prefixed mint skip to `tag_bnode2` while the unprefixed mint uses `bnode1`. Document deterministic prefix/stem/counter candidates and full-label collision skipping, retaining the caller's prefix guarantee. Include SERVICE in the shared allocator stem/category descriptions.
- MEDIUM: SERVICE module/request/resolver documentation promises a `SELECT *` spelling. Hygienic explicit projections and zero-visible unit transport columns are valid complete standalone SELECT carriers. State the actual carrier/ingestion contract.

One coherent public-contract documentation commit will correct these descriptions, validate the existing prefixed-collision/SERVICE controls and warning-free Rust documentation, pass normal hooks, push, and report results to issue and PR. Apply it after the ongoing assembly-generation pass so compilation reads a stable source tree.

These are documentation changes, not new allocation/carrier behavior. Retain all historical runtime benchmark samples and capture hashes. Reconcile the report/PR claims against the resulting source: explicitly record any documentation-only source drift instead of claiming every file still hashes identically or presenting a new measurement that did not run. The complete final assembly report and hosted checks must bind the resulting final source identity before merge.
