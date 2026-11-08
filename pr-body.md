Columnar preparation and decoding previously expanded shared nested terms into owned trees, including deep clones on memo hits and during dictionary construction. Both paths now retain flat scalar records and dependency IDs. Exact bottom-up identity memoization skips equal shared children; canonical sorting still uses the original structural RDF term order. The reader interns terms through the existing native factories in dependency order and finds reifier targets by their existing dictionary component IDs.

Parquet bytes, original source failures, wire ordinals, strict input ordering and refusal precedence remain unchanged. Core exposes the additive borrowed `ShallowTermKey` so both representations use one scalar-order implementation. Invalid triple-valued datatypes retain a lossless `ProjectionTermGraph` through the additive `DatatypeGraphNotIri` refusal. Its borrowed native records, reference IDs and root remain inspectable after source destruction; no production diagnostic expands a shared tree. Only the reachable operand
closure is retained, excluding unrelated earlier terms; local reference IDs are
remapped deterministically. Scalar datatype refusals retain their existing native operands, and both representations preserve the same diagnostic identity and human message.

Validation passed after correcting the invalid-datatype path: 43 columnar library cases, three unchanged byte-golden/union cases, 11 structured-error cases, strict release core/columnar library Clippy, and strict release codec-benchmark Clippy. The resource case, helper hygiene and normal signed commit hooks passed. Independent combined source review identified that diagnostic expansion and passed the corrected implementation with no outstanding findings. The existing codec benchmark now includes many distinct roots sharing a nested suffix. The synchronized head and reachable-closure correction passed the expanded existing native writer case, strict release library/test Clippy, independent source review and normal signed hooks. Hosted qualification passed: 48 successful checks, four conditional skips, one neutral notice, and no outstanding review debt or unresolved threads.

Resource cases exercise `write`, `write_to`, `project` and `read_with_ordinals`, reordered source IDs, valid object chains, hostile diamonds and depth refusals. At depth 16, 64 and 256 distinct roots sharing a suffix round-trip successfully with 145 and 529 distinct records:

| Phase / roots | Allocations | Requested bytes | Retained bytes | Peak bytes |
| --- | ---: | ---: | ---: | ---: |
| Prepare / 64 | 484 | 99,702 | 14,570 | 59,134 |
| Prepare / 256 | 1,781 | 376,274 | 53,270 | 222,554 |
| Read / 64 | 856 | 228,865 | 12,590 | 65,029 |
| Read / 256 | 2,372 | 834,534 | 40,235 | 225,773 |

Structural comparison steps for those cases were 1,833 and 7,508. Storage follows distinct records and edges; sorting remains depth-sensitive. At provider-chain depths 64 and 256, preparation retained 5,312 and 19,136 bytes, with 10,255 and 234,451 structural comparison steps. Decoder peaks were 32,591 and 114,551 bytes; those deep-chain decoder measurements include the required final depth refusal.

The issue's unrestricted diamond example is not a native-valid RDF dataset under the current subject-position rules. Native-valid chains through the existing depth limit round-trip; hostile diamonds and excessive depth retain their final typed reconstruction refusal. This change preserves those policies.

Closes #1181.
