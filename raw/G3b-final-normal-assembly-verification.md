# Final ordinary assembly qualification

Normal PR CI run 37537944878 has all seven independent simd-asm-config jobs
SUCCESS and aggregate job 112525965408 SUCCESS. The complete jobs snapshot is
G3b-normal-jobs-progress-3.json; native artifact metadata is
G3b-normal-artifacts.json. Exactly seven nonexpired simd-asm artifacts identify
the signed final head d48be7c960adfeaba44d642630cde4a66d7e86e7. Each downloaded
ZIP passes its native SHA-256 digest check. Each ZIP contains only its named
configuration JSON, whose bytes were read directly into the owned reports
directory. Original ZIPs/path inventories and per-report hashes are retained.

All seven reports are passed, each has its expected single configuration, and
their combined configuration roster is exactly complete. Every report's full
identity equals the successful hosted projection identity: source
fe73fce1722108d47b7fcb7a8bcbf6214318a23800304c759282753c41b005c0,
manifest c96ab1574d88cae967d67bcb642a7a4b7716555aaf9ca66e1b2e87fac36f760a,
schema 1, actual rustc EA137335/LLVM23.1.3. Every site's measured cells in each
ordinary report equal the independently qualified full projection column.
The fail-fast exact JSON correspondence check exits 0 (retained log true).
No local compiler, driver, toolchain selection or substitute matrix was invoked.

The actual aggregate job log says `OK: 110 sites; seven matching, successful
assembly reports`. Both aggregate and v3 checkout logs identify actual source
synthetic merge a582b06e0789d9fc29d71d7426846f56bda56382; the earlier runner
implementation `Commit:` banner is not source identity. Native Git commit API
confirms tree 5d150fa4076cc75e293b517f79dd080b6ea807ed and ordered parents
b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac / d48be7c960adfeaba44d642630cde4a66d7e86e7.
Those exact fields pass the fail-fast assertion and equal root's clean candidate.
G3b-ci-merge-identity.json records necessary projected provenance with its exact
origin; unrelated commit prose is omitted.

This closes final ordinary G3 assembly qualification on the actual final source,
compiler and candidate. Other normal CI jobs still run. Whole-issue completion,
final feedback/audit/Stage3 gates, merge/archive verification and cleanup remain
unverified; these assembly passes do not waive those obligations.
