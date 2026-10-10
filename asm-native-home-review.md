# Migrated original assembly homes

Root's Stage-only proposed manifest is simd-asm-manifest-native-proposal.toml.
No shipping file was edited by this reviewer. The proposal starts with the
writer's current manifest, including the existing scan_with correction.

| Old selector | Actual production body | Source evidence |
| --- | --- | --- |
| NumericFold::step_xsd | NumericFold::step_parsed | Original per-row fold accepts ParsedValue and original native workspace |
| NumericFold::combine_owned | NumericFold::combine_admitted | Original exact chunk-partial merge retains native parsed-value ownership |
| search::search_layer, both arithmetics and cache site | search::search_layer_admitted | Resident wrapper delegates to the admitted two-heap traversal and original query-owned admitted fixed-key cache |
| TextSearchRelation::holdings | TextSearchRelation::holdings_owned | Actual native supplied-candidate point scorer |
| normalize_whitespace_collapse | simple::is_collapsed | Public normalizer and borrowed native formatter call this original sixteen-byte clean-path precheck, which now owns the outlined SIMD work |

asm-native-home-evidence.txt records the actual emitted wasm32-simd128 paths and
measurements from the original affected qualification artifacts. Both numeric
fold functions, both HNSW walks and text holdings have zero vector/FMA/relaxed
work. The collapse precheck has8 vector operations, including i8x16.eq and
v128.any_true, and zero FMA/relaxed operations.

All original vector floors, required mnemonics, FMA ceilings, relaxed-instruction
bans and target configurations are preserved. Actual all-target qualification
and measured documentation regeneration remain required; this is not a gate PASS.
