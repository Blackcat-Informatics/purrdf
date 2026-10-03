HIGH assembly-audit coverage gap fixed in `28744b104af37faeb43a25bf865c25fa7094bc4d` (normal hooks, pushed).

The `scope_checks` benchmark now maps to its actual shipping `scope::validate_pattern` call chain. The new site records measured whole-symbol work without claiming a SIMD speedup; its observer tree walk has zero vector, FMA and relaxed operations on all seven configurations. All 102 prior site settings, build graphs and measured rows remain unchanged.

Validation passed:
- Stable full `make simd-asm SIMD_ASM_ARGS="--write-doc --jobs 2"`: 103 sites, seven configurations, exit 0.
- Separate final-source `make simd-asm` with `--jobs 2 --report`: exit 0; report identity matches the committed source/compiler, with all seven configurations and 721 site cells.
- Driver self-tests: ten runtime tests and all refusal/valid-neighbour controls pass; 154 generated documentation claims and `git diff --check` pass.

The latest remote base `97769c0d95026d95211768f0c30ce0c70c7309b3` is synchronized without conflicts. Complete final-head local gates and hosted CI/review are running; their outcomes are required before merge and are not credited from older heads. The earlier source-change-refused generation remains preserved as a failed attempt.
