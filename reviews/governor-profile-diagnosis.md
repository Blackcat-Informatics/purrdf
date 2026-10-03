<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Governor conformance diagnosis for issue 387

The current arena accounting is correct; the first-party profile identity and normative evidence are stale. No production rollback or aggregate scratch-credit transfer is justified. This diagnosis is read-only apart from the separately authorized focused test in `governor/charge_points.rs`. No build or test execution was performed by this reviewer.

## Production cause

At baseline 36122cc1ffd5580d3368dfad7aa8afaab4519a2d, `EvalCtx::charge_scratch_growth` compared arena minted bytes against the shared governor's total ScratchBytes consumption. The aggregate survivor buffer and custom accumulator state are charged independently through `ctx.charge_amount` (`modifier.rs:1740`, `:1923`, `:1986`, `:2027`). Those real charges are not payments for the final arena-owned result. Nevertheless, the old global comparison treated them as credits and concealed the final result's charge.

At d6f94 the arena-specific charged watermark correctly charges only that arena's as-yet-unaccounted retention. It preserves independent UDF arenas, cleared/replaced scratch, new governor attachment, and identity-label reservations. An aggregate result must therefore add its own deterministic term charge after the retained inputs/state. Restoring the shared-total comparison would reintroduce known governor evasion.

## Independent arithmetic

Each integer literal contributes its 40-byte datatype IRI, 32-byte deterministic term charge, and lexical bytes. These are proxy charges, not allocator-measured heap bytes.

- Current first-party governor fixture built-in SUM inputs 1..12: nine one-digit and three two-digit values = 879 retained bytes; SUM 78 = 74 result bytes; complete metered scratch = 953.
- Custom SUM over the same inputs: 879 retained + 64 declared state + 74 result = 1017.
- Custom wide SUM inputs 0..1199: 90090 retained + 33 accumulator charges of 64 = 92202; SUM 719400 = 78 result bytes; complete metered scratch = 92280. Its new boundary is 92280 and over-bound is 92279, derived by the harness, never typed into expectations manually.

Bounded fuel runs can latch a fuel trip before the final scratch checkpoint, so the actual regeneration diff must be reviewed case by case. A blanket addition to every spend or sweep line is incorrect.

## Profile contract and update procedure

`docs/SPARQL-GOVERNOR-PROFILE.md:636-643` and `governor/mod.rs:1414-1420` require a profile-version increment whenever charged event counts or ordering could move a caller's trip point, even when the fuel-price table is byte-identical. `vectors/sparql-governors/README.md:13` identifies this corpus as first-party, not vendored. The harness (`governor_corpus.rs:118-129`) and README (`:434-448`) explicitly document measured regeneration, freeze-manifest refresh, and library corpus-digest re-pinning together with a version change. This is a deliberate versioned contract change, not relaxation of a failing oracle.

1. Bump `GOVERNOR_PROFILE_VERSION` 9 to 10 and describe the corrected per-arena accounting/identity retention. Leave all 17 unit fuel-price entries unchanged. The independently recomputed v10 schedule digest is `d1a2df1c68427c65add1e1d9279bb0d252290f921be8086384b4178531921ea8`.
2. Update profile section 3 scratch dimensions/checkpoints, section 10 version/recipe/digest, section 12 version history and consumer remeasurement instructions, and section 13 pinning fields. The scratch description must include separately retained identity labels, arena values, aggregate buffer clones and declared custom state using deterministic charges, without claiming actual allocation measurement.
3. Run the corpus's documented measured regeneration under `PURRDF_UPDATE_GOVERNOR_CORPUS=1`, then separately regenerate the ignored fuel-sweep trace at `crates/sparql-conformance/tests/goldens/evaluator-trace/governors.trace`.
4. Refresh the first-party governor freeze manifest through the authoritative tool and re-pin `GOVERNOR_CORPUS_DIGEST` from its SHA-256 in the library and profile section 11.1. Verify every other freeze manifest remains byte-identical.
5. Review complete spend/outcome/answer/metered/charges/trace diffs against independent accounting, then rerun without regeneration environment and complete all required gates. Three first mismatches in the failing harness are not the full change inventory.
6. Publicly amend the accepted plan's broader no-frozen-edits wording for this first-party versioned evidence repair. Preserve all frozen official corpora and externally governed GTS bytes.

## Focused regression handed to root

`aggregate_retention_cannot_hide_the_finished_results_scratch_charge` is implemented in `crates/sparql-eval/src/governor/charge_points.rs:1921`, formatted, and source-stable. It reuses existing evaluator fixtures/helpers and independently pins 878 retained bytes for integers 0..11 plus 74 result bytes for SUM 66 = 952. Both built-in SUM and registered custom SUM with state bound zero must report exactly 952 and answer 66. Scratch ceilings 951, 952 and 953 must respectively report a typed scratch budget trip (consumed 952), complete, complete. The direct context check pins arena-owned growth 74, aggregate-plus-arena total 952 and unchanged totals through two repeated checkpoints. No validation pass is claimed until root runs it.
