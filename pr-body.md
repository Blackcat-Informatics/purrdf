Imported inconsistency proofs failed when the CLI exported the resolved closure as canonical N-Quads and reparsed it for checking: canonical ordering changed the first-seen native term IDs. Check supplied proofs against the consumer session's resolved snapshot instead, constructing a fresh checking context through the shared verifier.

The original inhabitant/premise checks remain intact. Add native and scalar-WASM controls for independently loaded closures, scoped blank labels, derived membership and changed-source/answer/proof refusal. Canonical data export and proof bytes are unchanged.

Validation: the unchanged failing CLI case, two native public harness cases (including sixteen frozen vectors), five native current-support controls, both exact scalar-WASM cases, affected strict Clippy and standard helper hygiene pass. Normal hooks are recorded with publication. Hosted qualification remains required. No full local gate or external corpus was run.

Owner review only locally; no independent approval is claimed. Wider consumer combinations remain the principal uncertainty for hosted CI.

Closes #591
