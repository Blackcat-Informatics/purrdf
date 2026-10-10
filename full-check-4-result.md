# Original mandatory gate terminal receipt

Command: `bash /opt/purrdf-schema-499-qualification/run make check`, with its
complete stdout/stderr in `full-check-4.log`. Original four compiler/test jobs,
16GiB memory cap, zero swap and pinned Stage toolchain; no gate, hook or profile
override.

The authoritative unified execution handle was 67258. Its final `write_stdin`
result reports **exit_code: 0**. The unchanged complete mandatory gate is PASS.
The final output is `Finished release profile [optimized] target(s) in 6m 46s`
after the full release-WASM build. The original release-census check in that same
gate confirms all32 publishable crates; no member or suffix is excluded.

Preceding strict workspace/consumer lint and compilation, original hygiene,
generated/tooling/corpus gates, whole-workspace unit/integration tests and
doctests, preserve-order consumer test/doctest, and kernel/ring-fence all passed.
Source was frozen throughout; subsequent staging changes no source bytes.

Historical full attempts1/2/3 remain failed records with their concrete
corrections. Local qualification does not establish hosted CI, integration or
issue closure. Independent whole-contract disposition is owned by the reviewer.
