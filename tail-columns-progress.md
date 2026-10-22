Internal checkpoint `bf6fab38b` is committed and pushed with normal signed hooks.

Parallel relation/key/free-slot arrays and mutable tail columns now allocate under the original supplemental admission. Existing sealed arrangements retain their original owners. New-partition buffers remain in the original store across refusal, so retry reuses accepted column growth. Retirement slots are admitted before publication; empty-partition retirement allocates no storage and queues exact destroyed column charges.

The original-watermark refusal control now adds an unrelated new partition alongside a productive input. Repeated refusal preserves native joins, retained buffers and the original pending watermark, then produces the exact five facts, two witnesses and three membership changes.

Qualification: 455 native Datalog library and 45 public cases, all 12,608 cumulative operation cuts at 12,607 units; all eight extended original-owner controls; 818 native entailment library and 16 public cases (one existing ignored); all 45+16 actual packaged WASM public cases; all-target Datalog/entailment/SHACL/validation Clippy with warnings denied.

Supplied constructors remain private. Sealed batch/log/object and compaction allocation, physical factory work charging and the remaining full parent qualification are unfinished. No parent PR or completion claim. Next implementation reserves those publication buffers before a row is published, reusing the original merge algorithm and tier policy.
