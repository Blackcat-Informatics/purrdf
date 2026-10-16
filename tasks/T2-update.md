Byte-aware dictionary packing is pushed in commit `4b031fbac`. The existing
front-coding encoder closes before the next record would exceed the byte limit;
the record count is a cap. Forward/reverse streams may have different populations
and block counts. Header admission, complete certification and bounded term lookup
consume that layout together. A single oversized encoded record still returns
`ConstructionLimit`; no whole-image retry or eager lookup directory is added.

All 17 segmented and 15 stack integration cases pass, including long alternating
terms, high IDs, sparse reads, actual unequal dictionary/reverse block counts,
RDF 1.2 side tables, streamed output, single-record overflow and existing ownership,
refusal and source-fault controls. Targeted units, affected strict Clippy and
refreshed helper hygiene pass within their recorded scopes. Normal hooks pass.
All 17 segmented cases also execute successfully in scalar WASM.

SIMD WASM is running. The existing real file-backed evaluator control now requires
4096/128 to succeed, at the original 2,000 and 200,000 claim populations, alongside
its LIMIT, bound-key and full COUNT checks. That execution and final complete-issue
qualification remain before the PR. No timing acceptance is used.
