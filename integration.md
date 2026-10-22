# Integration assessment

Head eae70d3b675ae66ed7190f5a2a21935785bb2277; actual fetched base 09e344f7338a1ec4ac312d5feb9f469ceb7234e3. git merge-tree --write-tree --no-messages origin/main HEAD exits 0 with tree 4228f981f423740ee36c4be61280b7d8f8bee482.

The PR was created against 68d5549936e83608eda517fde81b2640f318b69c. Its hosted workspace job checked out pull/674/merge at 38266ca15c88e036d65b29ba3ebd372608628853 before PR 672 landed. That older test merge does not qualify the complete current combined tree.

The intervening base delivery changes GROUP worker result retention and typed row admission in sparql-eval parallel.rs, modifier.rs and row_checkpoint.rs, plus BENCHMARKS.md. This delivery changes only columnar source-error transport, its test development edge, portable test registration and API documentation. Columnar's production dependency closure does not include the SPARQL evaluator; no altered evaluator function calls columnar project/write. Neither delivery changes DatasetView, ReadError, StructuredError, shared allocation admission, dependencies used by the other production path, toolchain, profiles or generated artifacts. Makefile changes here register columnar cases only. The generic error wrapper remains inside columnar APIs and no evaluator signature consumes it.

Thus the base delta does not invalidate the native/portable columnar results, compiled Python/facade callers, strict lint/census checks or grouped-buffer acceptance. The source merge is clean. Reuse those scoped receipts; no full combined-suite or fresh test-merge execution is claimed. No synchronization solely for branch age. Hosted checks and full feedback must be terminal before merge; refresh inputs immediately before ghprsq and compare its actual reported landing afterward.
