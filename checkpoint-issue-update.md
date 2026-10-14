Implementation checkpoint pushed: 80ec82890 (`paudley/490-typed-capacity-errors-instead-of-u32`). The normal commit hook passed. This is not issue completion and no PR has been opened.

The checkpoint includes typed native capacity propagation and rollback, 64-bit Datalog IDs/global text offsets, original host-ID/dictionary rule execution, and chronological sparse segmented publications with shared admission, lazy touched identities, retained evidence and atomic consumer-owned replacement.

Current focused evidence:
- Strict core Clippy and RDF/umbrella production library checks pass.
- Mutable runtime 51/51; native graph/rollback 21/21; capacity controls 15/15; public import and shared scope controls 1/1 each; term/global arena 26/26 and global ID boundary 1/1; complete Datalog library 367/367 pass.
- 48 actual raw/prepared SPARQL executions preserve solution bags/unbound cells; provider-fault evidence remains typed.
- Generated chronological parity passes 128 publication checkpoints against canonical eager typed seals. Encoded purge checks preserve untouched source bytes and old snapshot ownership, then find no removed lexical or datatype bytes after final old-owner release.

Remaining acceptance is explicit: portable operational/host routes and release WASM builds, full host/package checks, enforced non-test capacity-panic census, generated/manual projection reconciliation, mandatory repository gate and final complete-contract review. Build/toolchain/cache machinery remains unchanged. Both capacity and sparse-stack contracts remain open and will be delivered together.
