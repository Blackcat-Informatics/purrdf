# Corrected metadata caller: exact owned source pair

Root prepared shipping-source and dev-source clones under
`/opt/purrdf-308-current-telemetry.591np1/metadata-current` from the corrected
staged tree `a68fb1b716c5937af20b38b9930a69191d4cd6d4`. Actual preparation
session24620 exited0. Both materialized indexes initially matched the entire
imported source tree (`git diff --exit-code` actual0). The dev clone's sole
subsequent difference is the production counterfactual's existing debug-assertions
branch selecting `dev` instead of `test` in c_smoke.rs; full diff confirms one
string, one file. The shipping clone retains the current production selection.

Earlier ff72 snapshots and their passing individual routes/failed strict paired
comparison remain untouched and attributable. New Request JSON uses distinct
empty owned arm directories and the same concurrency/cache declarations. No
source, index, MERGE_HEAD or ref in the implementation worktree changed during
preparation. This creates no synthetic source commit.

The current matched C pair must settle before invalidation trials. The shipping
clone's actual Cargo-selected c_smoke harness can then exercise source/codegen
invalidation without another clone or no-run compilation, under the independent
review's preservation and exact-restoration requirements. Keep every baseline
receipt and copied timing file unchanged, write trial evidence to separate paths,
and revalidate the original strict pair after restoration. No changed warm-arm
request or weakened comparison is authorized.
