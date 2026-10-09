The telemetry repair is committed and pushed as ca713d3fe, including the resolved current-main integration. Normal commit hooks passed.

The collector now parses real mixed Cargo/libtest output through the shared strict frame parser, captures each selected C library before its owning caller removes temporary files, and binds the actual C run to exactly one child artifact. Cargo metadata receives the same admitted jobs/target/build environment as the measured commands; the full inventory and strict comparison remain intact.

Local qualification passed 29 focused tests, strict CAPI all-target clippy and the owning hygiene/profile/shard gates. Both current-source C arms completed all 16 phases and passed the strict comparison. Nine direct existing-harness trials passed all 16 phases each, proving source and effective codegen invalidation, unchanged reuse, exact restoration and preservation of the original comparison evidence. Independent repair review passed.

A fresh complete twelve-arm cold/warm campaign is now dispatched: https://github.com/Blackcat-Informatics/purrdf/actions/runs/37762276551

Earlier failed campaigns remain failed and retained. Numerical improvement, the new complete hosted comparison, the settled full local qualification and final completion audit remain unverified; this is a repair progress update, not issue closure or PR readiness.
