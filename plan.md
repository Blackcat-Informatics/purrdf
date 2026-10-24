# Assembly artifact lease

Behavior: hold the installed Cargo cache activity lease for the complete assembly CLI invocation, including compilation and analysis. Nested Cargo reuses its ancestor's lease. Portable hosts without the Stage coordinator retain their existing workflow.

Dependencies: the shared cargo-cache-lease command from the Stage cache reclamation delivery. The coordinator must be deployed before enabling forced recovery.

Exclusions: no assembly measurement, parser, Rust, compiler, target, model or GPU changes. No workspace-wide validation added. Preserve the non-Rust line-count ratchet.

Acceptance: the 17-case assembly self-test passes through the real shared helper with a dedicated temporary cache. Required commit hooks, including the non-Rust ratchet, run fully. Independent combined review covers this entry point with the Stage coordination code.

Risk: a reader lease released before analysis would let cleanup remove evidence. The CLI-level lease spans the complete invocation; its nested commands reuse the same lifetime.
