# Correlated native traffic correction

Status: SOURCE PROPOSAL READY; compilation and runtime acceptance NOT MET.

Ordinary packet: `correlated-delivery-lookup-owner-draft.patch` (five original source homes). Full postimages and pre/post hashes are adjacent. `patch --batch --dry-run -p1` passed all five homes; rustfmt parsed/formatted all Stage postimages. No shipping edits, compiler, tests or build ran in this helper lane.

## Actual failure and selected correction

Writer observation `implementation-native-owner-correlated-traffic-observation.log` proves query 0/direct route stops at the original 16,384-request ceiling with 16,380 retained requests and 1,818 evictions. The dominant ranges are repeated reverse-index reads. The original seven-query/four-route fixture and its ceiling/physical/lifetime assertions remain unchanged.

`eval_join_delivered` previously bypassed the admitted singleton-VALUES seed because active delivery always entered `eval_binary_yielding`. Both bulk and delivered restriction recognition now share `eval_values_restricted_join`. BGP already materializes before RowDelivery; its restricted bag is evaluated through the original node checkpoint and `eval_bgp_seeded`, reordered through the original admitted row home, and delivered exactly once. Compound streamed children retain their original incremental delivery order. The leaf path in `finish_seeded_join` uses the BGP compiler's own real bound mask and order instead of constructing a redundant positive-region forecast. Compound positive regions still use their existing plan.

## Identity and allocation ownership

Bounded EvalCtx owns a lazy `TermLookup<D::Id>` with original admitted shared control, immutable WorkspaceTerm keys, admitted u64-hash table and exact-equality collision buckets. Successful native ids and genuine absence are cached; source/physical failures are not cached as absence. No host id width conversion or join-path remapping occurs. Worker and same-view function contexts share the original cache owner. Resident production uses its existing reverse-read path and creates no new cache heap owner.

Forward native resolution in `outer_bindings_for_substitution` records the already-proven Existing id before Values Insertion reconstructs its lexical constant. The original checked scratch promotion/store body accepts the lookup callback, retaining its language gate, computed-identity precedence and query-scoped blank rules. BGP execution constant compilation and rdf:reifies resolution use the same query lookup. Standalone preparation forecasts/reference compile doors retain their original source-read/latch boundary and call order. Bounded rdf:reifies lexical construction is admitted, avoiding first-use unpriced OnceLock string birth; resident callers preserve that existing static path.

Every cache key copy is `clone_term` before allocation, every bucket/table uses admitted native containers, and shared control uses `SharedWorkspace::new_admitted`. Exact term hashing/comparison uses the original workspace/native kernels. Payloads die before their original grants; last query/worker owner frees the cache. Cache growth failures remain the account's original typed first refusal. No trace increase, fabricated byte fee, postconstruction census or raw extraction is introduced.

## Qualification required in the writer lane

Three same-home memo fixtures cover known-id reuse/shared worker state, actual equality within a deliberately collided bucket, and failure-versus-absence. Their resident capability explicitly exercises the admitted kernel for semantics, not a physical-bound proof.

Run the unchanged complete correlated public fixture across seven queries and four actual entry routes with the original segmented request cap; evaluate library/reference/governor/source-error gates; segmented/HNSW and prepared pins/query completion prove dependent native source and retained-account behavior. The previous HTTP/SERVICE fixtures must remain passing. Report actual source traffic and peak/lifetime evidence from this integrated identity; do not infer runtime success from this proposal or apply/syntax results.
