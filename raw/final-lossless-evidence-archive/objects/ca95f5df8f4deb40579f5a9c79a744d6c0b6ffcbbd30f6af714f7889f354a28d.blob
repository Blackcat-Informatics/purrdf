# Actual conformance shard reuse and scoreboard regeneration

Read-only assessment; no generator, suite, artifact download, dispatch or source edit executed. Aggregate113454114331 in run37817482468 is genuinely FAILED with Make2. Its actual log `tasks/current-hosted-ed14/job-conformance-aggregate-113454114331.log` reports all semantic rows GREEN but refuses the stale generated docs block. Exact diff is Python binding pass4164→4192, with four ledgered strict-xfails unchanged; LSP81/five strict-xfails is unchanged. The log proves measured aggregate counts; it does not replace retained raw shard artifacts and their job/source identities.

Supported production route: four workflow `conformance-shard` jobs (`core`, `sparql`, `shapes`, `python`) run `make conformance CONFORMANCE_ARGS="--shard NAME --emit-results target/conformance-results/NAME.json"`. Noncancelled uploads retain one `conformance-results-NAME` artifact each. The aggregate downloads those same-run artifacts and invokes `make conformance CONFORMANCE_ARGS="--from-results target/conformance-results"`, then separately requires all shard jobs succeeded.

The existing `scripts/conformance-matrix.py` supports `--from-results DIR --write-doc` together. This invokes its normal scraper selftests and full registry ownership checks but does not invoke suite closures, rebuild the native module or rerun Python. `merge_shards` requires exactly core.json/sparql.json/shapes.json/python.json, format `purrdf-conformance-shard/1`, correct shard names, exact ordered owned row indices and every SuiteResult field. It then applies the shared budget/orphan/XPASS law over the complete matrix and writes only the owning generated doc block. No custom merger, manual count or native-only doc projection is justified.

Root's exact bounded repair sequence, after artifact retrieval admission:

1. Select only the four named successful shard artifacts from run37817482468's attributable attempt; retain artifact IDs, download digests, job terminals and exact checkout source. Extract into one fresh owned directory with no overwrite or stray file. Retain raw ZIPs and individual file digests in Stage evidence. The payload format has no source/compiler identity fields, so names or a successful structural merge alone cannot establish provenance: bind those externally to the actual same run/head and shard logs. Never combine other attempts or runs.
2. Require actual successful source-bound shard jobs and inspect each raw report. Then, in the current repaired worktree, run the existing production generator only:

```bash
make conformance CONFORMANCE_ARGS="--from-results /opt/OWNED-CONFORMANCE-REPAIR/results --write-doc"
make conformance CONFORMANCE_ARGS="--from-results /opt/OWNED-CONFORMANCE-REPAIR/results"
```

3. Preserve both actual command exits/full output, original doc preimage and generated diff. Verify only docs/CONFORMANCE.md's owning matrix block changed and counts match the actual4192/4 and81/5 observations. The generator must not be replaced with a new private Python checker. Its existing supported legacy entrypoint is used without code growth.
4. Normally commit/push the source-bound doc projection and require current-head mandatory CI. Old aggregate failure remains historical; regenerated scoreboard parity is not final-head CI PASS.

Applicability to corrected1905: the only delta from qualified72fa shipping source is three explicit `dataset_required:false` initializer additions inside cfg(test) structural visitor fixtures. Their owning tests are not new Python cases or conformance registry entries; they restore compiling test IR without changing production bodies, corpus bytes, budgets, matrix driver/registry or Python suite/test inventory. The old ed14 four-shard measurements therefore remain reusable for this doc projection after exact source-contract comparisons and artifact admission. The upcoming repaired committed head still needs new mandatory CI; old shard reuse cannot certify a changed generator, corpus, Python inventory, production binding or budget. Any actual unexpected source difference blocks this reuse assessment.
