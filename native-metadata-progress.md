Internal checkpoint `36063bee0` is committed and pushed with normal signed hooks.

Fresh sparse row-log, primary partition lookup and canonical directory metadata now allocate under the original supplemental admission before replacing their Resident predecessors. Actual native insertion, restoration and directory growth use that same owner. Independent clones construct their own Resident metadata.

A real refusal regression exposed a progress bug: discarding an unfinished directory also discarded accepted allocations, so the small original model could fail to finish. Preparation now retains the unfinished directory and next-key cursor until publication. Destroyed predicate arenas retain their original charge until release succeeds. Final transaction completion now precedes destruction of every final task/label array, so release retries preserve completed work as well as rows and the original watermark.

Qualification:

- Native Datalog: 455 library and 45 public cases, all 12,608 cumulative operation cuts at 12,607 units.
- Eight original-owner refusal controls pass; the bounded directory regression genuinely failed before the retained-builder fix.
- A direct native metadata control passes actual row-log/lookup/directory growth and refund refusal, independent clone ownership, original IDs and empty-predicate arena retirement refusal.
- Native entailment: 818 library and 16 public cases; one existing ignored test.
- Actual packaged WASM: all 45 Datalog and 16 entailment public cases.
- All-target Datalog, entailment, SHACL and validation Clippy passes with warnings denied.

Supplied constructors remain private. Parallel free/key/relation arrays, object/column storage, sealing/compaction, physical factory work charging and the remaining full parent qualification are unfinished. No parent PR or completion claim. Implementation continues with the parallel store arrays under this same owner.
