SHACL parsing and preparation could repeatedly copy shared definitions and exhaust the native stack. This change retains original definition/function domains, parses definitions once, and carries identity references through preparation, validation, reports, rules, schema projection and prepared products. Shared read footprints use an arena and memoized path pairs. Artifact identity hashing streams its canonical framing without repeatedly allocating serialization buffers.

Caller-authored recursive shape catalogs and exhausted physical-stack margins produce typed refusals. The bundled structural meta-schema keeps its existing bounded evaluation. The complete explicit LFP/GFP solver required by #569 remains open. Earlier ShEx commits introduce no net ShEx diff against main.

Scope is frozen to the work published at 2e42e8792 plus review and compatibility repairs, at the user's direction. Repairs are pushed through ea133b288; regressions and their evidence are tracked in #690.

Validation so far:

- Normal commit hooks passed without bypasses. All local Cargo calls, including oracle subprocesses, use the configured wrapper.
- Core library: 1398 passed on unchanged core inputs. Shapes library: 989 passed; the final LinkML delta passes all 41 focused library and 139 projection controls. The earlier 29 ontology-consumer controls remain applicable.
- All 14 originally failing integration targets now have passing focused rechecks. All 65 product-refusal, 41 restored-expression and 29 real CLI controls pass. This reuses unaffected results from the broader integration run; it is not a fresh complete make check claim.
- Shapes/CLI all-target Clippy passes with warnings denied. Full helper hygiene passes: 87 enforced jobs, 121 exact distinct groups and 150 unique prefix-free hash domains.
- Official TypeScript 7.0.2 and GraphQL.js 16.14.0 oracles pass exact/projection probes, located losses, value handling and verified reverse import. The GraphQL wrong-kind acceptance regression was reproduced and fixed by emitting a plain reference instead of a one-element intersection wrapper.
- The unchanged official LinkML oracle passes all exact, located-loss, rename/reverse-import, list, temporal and value-shape probes. LinkML output retains anonymous slot alternatives where named type/choice declarations lose validation. It borrows original bodies, preserves sibling conjunctions, guards cycles/stack margin and limits expression count by source bytes. The exact member pin retains both original alternatives and body/cardinality assertions.
- Product admission measures 230 allocations versus the former 293. Cold static-root preparation measures 18 allocations/3106 requested/0 additional retained/1674 peak bytes at 1/8/32 retained definitions. All 12 allocation and 7 catalog controls pass without raising original ceilings. Incremental footprint and carried analysis pass 18 and 10 controls respectively.
- The supported current-stage generator wrote the 5608-byte fixture. All 7 ordinary product-determinism and 28 census controls pass; the frozen format-epoch artifact is unchanged.
- Main was merged normally at the user's request; its delta was README prose. Final-head CI run 40041027523 is completed/success on ea133b288, and the full PR rollup passes, including exact Rust 1.98, Python, all native/WASM/cross-architecture and seven assembly configurations plus aggregate gates.
- The four recorded inline findings and repeated planned-lookup nit are fixed, with tested replies and resolved threads. Final-head independent review reports no actionable comments; general risk proposals were independently withdrawn after source/baseline reconciliation. Five assembly documentation cells match actual hosted production-symbol measurements; manifest thresholds are unchanged.

The review's general cardinality concern is covered by the unchanged pre-parse guard and 12 native controls. Its OpenAPI packaging concern is a verified pre-existing bug: the emitter function and main's frozen OpenAPI golden are unchanged. That defect is tracked separately in #692 under the frozen scope.

Related: #569, #690, #692. None is closed by this PR. Current required CI and review reconciliation are complete.
