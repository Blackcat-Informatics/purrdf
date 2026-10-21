# Python lock update acceptance

The complete source delta changes only certifi in bindings/python/uv.lock: version 2026.6.17 to 2026.10.19 with its new sdist/wheel URLs, SHA-256 hashes, sizes and upload metadata. Dependency edges and all other packages are unchanged. Independent source review of the dependency author's patch found no actionable defect.

Exact PR head 3bd477ebf has passing full CI, including Python conformance/pytest, package builds, workspace, portable tests and seven assembly lanes. Four manual/profile jobs are configured skips and CodeQL is neutral. pr-checks.txt lists every result; no waiver is used. Local uv lock --check --project bindings/python exits0 on CPython3.13.12 and resolves107 packages without changing the lock (raw/lock-check.log). No duplicate full local suite ran.

Integration against fetched origin/main9965023bd is clean: merge-tree exits0, tree6a41f9c9ab2b82fc6cf70a047790671ba426f0b6. Intervening base changes concern Datalog traversal/continuation and assembly documentation, with no Python dependency/consumer changes. PR-head qualification and the lock check cover this independent update; no routine synchronization or restarted CI is required.

Setup reused stagectl's exact returned paths. The clean generated branch was switched normally to the existing Dependabot branch; open.json remains the original receipt. Explicit PR659 is used for forge operations. Source is clean. Final feedback/check/base refresh, ghprsq archive and landing readback remain required.
