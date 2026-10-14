# Temporal governed admission checkpoint

Source: dirty #470/#443 worktree at HEAD `2a1a15dcef2b97d87f0af29a208253db0b7df596`, including `temporal_parse_bytes`, temporal lexical preflight, and `try_parse_with_memory` temporal admission. This is a focused checkpoint, not a commit or full issue qualification. No exclusive timing was used.

- `cargo test --locked -j 8 -p purrdf-xsd --no-run --message-format=short`: terminal 0 after correcting one unnecessary-qualification diagnostic.
- `cargo test --locked -j 8 -p purrdf-xsd --lib temporal::tests -- --test-threads=4`: terminal 0, 82 passed.
- `cargo test --locked -j 8 -p purrdf-xsd --test exact_governance wide_temporal_parse_admits_native_peak_and_releases_retained_owner -- --exact --nocapture`: terminal 0, 1 passed. The control measured wide Time, DateTime, Duration D/H/M/S, and dayTimeDuration native peak and retained storage; each refused at one byte below its computed admission before any allocation, and released its retained owner.

The preflight's fractional-scale check was added after these commands and awaits retest. The measured shapes do not prove the symbolic bound for all lexical lengths or all calendar families. Temporal canonical rendering, exact-only downstream consumers and the other issue contracts remain open.

Later checkpoint on the same dirty branch: after correcting the decimal coefficient-digit rule at `exact::cost` and making the duration reader's wide Mul/Add/Neg fallible, the same focused governed test ran terminal 0 (1/1). It included a 5,000-digit integral seconds field, a trailing-zero integral field, negative seconds and malformed-wide lexical neighbors. This later command was `cargo test --locked -j 8 -p purrdf-xsd --test exact_governance wide_temporal_parse_admits_native_peak_and_releases_retained_owner -- --exact --nocapture`.

Subsequent source edits to `split_seconds_day`, other temporal arithmetic helpers, and the recursive-product peak envelope have **not yet been compiled or tested**. These later edits invalidate this receipt as final-source qualification.

Latest bounded checkpoint on the same dirty branch, after the fallible arithmetic, recursive alignment product envelope, and admitted temporal comparator wiring:

- `cargo test --locked -j 8 -p purrdf-xsd --no-run --message-format=short`: terminal 0.
- `cargo test --locked -j 8 -p purrdf-xsd --lib temporal::tests -- --test-threads=4`: terminal 0, 82 passed.
- `cargo test --locked -j 8 -p purrdf-xsd --test exact_governance wide_temporal_parse_admits_native_peak_and_releases_retained_owner -- --exact --nocapture`: terminal 0, 1 passed, including balanced wide fractional scale and trailing-zero integral seconds.

The latest source still lacks a direct admitted-comparator refusal control, prepared temporal rendering, and complete calendar cast ownership. No full suite or host integration pass is claimed.
