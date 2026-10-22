Internal implementation checkpoint `d2e19593a` is committed and pushed with normal signed hooks.

The original supplemental admission now covers deferred publication, native task arenas and spills, event fan-out, preparation indexes, runtime weight growth, result deltas and term-reclamation buffers. Destroyed replacement buffers retain their exact outstanding charge when release refuses. Event cursors advance before retirement, and completed settlement remains readable per relation while the full snapshot and original watermark await final release. Rejected-batch cleanup retains its charge; later transactions reuse the admitted result-delta buffer.

Qualification on this checkpoint:

- Native Datalog: 454 library and 45 public runtime cases, including all 12,608 cumulative operation cuts at 12,607 units.
- Eight original-grant controls exercise real growth/spill/refund refusal, repeated retry, native publication, rejected batches, complete relation reads, watermark retention and latest-delta term pins followed by reclamation.
- Native entailment: 818 library and 16 public clash/refutation cases; one existing ignored test.
- Actual packaged WASM: all 45 Datalog and 16 entailment public cases.
- All-target Datalog, entailment, SHACL and validation Clippy passes with warnings denied.

This remains an incomplete internal path. Supplied constructors stay private. Sparse store/index factories, physical factory work charging, remaining guard/group/source/resource cuts, refreshed whole-owner allocation and scale evidence, full gates, independent review and CI remain required. No PR or parent completion is claimed. Next implementation is the remaining store/index allocation path under this same original owner.
