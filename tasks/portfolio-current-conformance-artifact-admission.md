# Current conformance artifact admission

Retrieval and verification PASS. Generator/source edit/build/dispatch NOT EXECUTED. Fresh exclusive retained root: `/opt/purrdf-454-conformance-ed14-20261008.FOVwkJBK`; results subdirectory contains exactly four nonoverwritten JSON files. Raw ZIPs, API metadata, complete job logs, extracted member lists, hashes, all32 row observations and admission outputs remain there. No global/sibling cache was changed.

Actual run API admission:37817482468, attempt1, head `ed14f23ead7a303a18b9ef610f1e6caeb4483a35`. All four specific jobs are success at this head/attempt:

| Shard | Job | Artifact | ZIP SHA256 |
|---|---|---|---|
| core |113449712033|11568571508|ef48a9d3d58797ebbb23e7770700ec441b61b987c2282335fc5a35eb80f9d4ff|
| sparql |113449713036|11568071926|9d5a957fb1d0a50507aa87efd9e2d7a7e7f1a08d15b50087ab804e79eff7a6c2|
| shapes |113449712088|11568016965|a6f147cce8a305e839e0a2594379800a5ac673e94571c5258bc27b74a54eeefb|
| python |113449712339|11567604116|5eee004e5d92b260799d7f3409ab53796583fcc4f1a7090a138e058129f52d24|

Every downloaded ZIP passed its API-provided SHA256. Each ZIP member inventory was exactly its shard.json before `unzip -n` into the fresh results directory; no traversal/extra entry was accepted. Metadata binds all artifacts to the exact ed14 head. Complete raw logs are `jobs/{core,sparql,shapes,python}.raw.log`, total196224 bytes; raw ZIPs total46726 bytes. `jobs-api.json` supplies actual terminals/attempt identities, not a guessed log-tail verdict.

Initial retrieval session36817 exited1 after four successful ZIP digest/extraction checks because gh refused log output containing terminal escape sequences. That CLI output refusal is preserved here as a retrieval limitation, not a job failure or accepted missing log. Corrected log retrieval through explicit `gh api --allow-escape-sequences` completed actual session86707 exit0; all four full raw log files are now retained. No semantic command was run.

Payload admission outputs (`schema-admission.txt`, `budget-admission.txt`, `run-admission.txt`, `job-admission.txt`) are all true, terminal0. Inspection found exact format `purrdf-conformance-shard/1`; expected top-level/row fields; core12, sparql9, shapes9, python2 rows; full disjoint indices0–31; every row ok:true/failed:0/scoreboard_missing:false. Each of32 exact suite names exists in current committed budget with exact xskip equality and no budget orphan. Original production aggregation remains the final owner of registry ordering and scraper/ratchet judgments; these artifact checks do not replace it.

`all-rows.tsv` retains every measured count. Python4192/4, rdflib LSP81/5, community dated SHACL115/0 are actual input observations, not manual doc edits. Existing aggregate log demonstrates full original registry ordering was accepted before doc drift refusal.

Current source HEAD is `6ce3cc6567d0874414cf138bcdfd11f4799e6901`. Actual `git diff ed14 HEAD` saved as source-delta.patch/source-delta.txt contains ONLY three added `dataset_required:false` lines in cfg(test) structural fixtures (reasoning, dataset_description, ownership). Therefore the complete generator/Make/workflow, registry, baseline budgets, all corpus/vendor files, shipped native/Python paths and Python test inventory are byte-identical between the measured cohort and current head. `current-source-inputs.sha256` records current owning inputs. No source reuse assumption based merely on issue titles is needed; these artifact measurements qualify the bounded docs-only projection at6ce. New mandatory final-head CI remains independently required.

Ready next command after root admission, in current worktree:

```bash
make conformance CONFORMANCE_ARGS="--from-results /opt/purrdf-454-conformance-ed14-20261008.FOVwkJBK/results --write-doc"
make conformance CONFORMANCE_ARGS="--from-results /opt/purrdf-454-conformance-ed14-20261008.FOVwkJBK/results"
```

Neither command was executed. Do not add these raw logs to published Stage evidence until its owning plaintext credential scan approves them. Preserve source/doc preimages, actual exits, generated diff and final head separately; old aggregate Make2 remains failed.
