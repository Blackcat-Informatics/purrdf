# Actual main integration: #499 with #508

Actual published #499 head before synchronization: `b7195b537f535b688c77c8dbd3ee9b97246e198c`.
Actual fetched main: `a77b743bcc6fad89d88581ef9cb5a626a326834f`.
Normal `git merge --no-commit origin/main` produced two real conflicts, both in the original exact numeric home (`exact/integer.rs`, `exact/decimal.rs`). This is an actual overlapping integration, not synchronization solely because the branch was behind. The merge remains uncommitted pending affected qualification and independent parent review.

## Complete resolution

- Retain #508's integer and BigInt native-copy/sign/arithmetic kernels, original checked layouts, admitted renderer, and public fallible operations. Remove only the duplicated #499 native kernels introduced by the automatic textual merge.
- Keep #499's internal `Mag` visibility required by original RangeMemory. Its numeric cloning macro continues to call the same canonical native-copy body; no second clone implementation or account is introduced.
- Decimal `cmp_using` and `round_to_integer_using` are thin range-storage adapters over #508's original comparison and rounding bodies. The independent comparison, rounded division, promotion and result rendering semantics remain in those shared bodies.
- Exact decimal exponent extraction now accepts the same selected native allocator. BigInt's original decimal-group division algorithm takes `Allocate`; its existing public default remains `Fallible`. Both the immutable groups and successive quotient magnitudes are created through the original allocator before physical allocation. This is necessary because a no-op physical phase callback in RangeMemory must not let a nested default allocator escape the original owner.
- The new public range regression compares unequal-scale, 513-digit endpoints with a known empty interval. It checks actual allocator allocation count against owner callbacks, actual peak against original admission, and zero retained/scratch bytes. Existing refusal/error-precedence fixtures remain unchanged.
- The affected compiler found a textual automatic merge collision outside the two conflict-marked files: #499's former `core::small::try_boxed` definition collided with #508's re-export of the same job from its new original `lex::allocation` home. Remove the obsolete core implementation and private error type. The original #499 Products box admission still precedes the same fallible boxing job through the core re-export; its original typed failure mapping and retained control layout are unchanged. Main's actual DatasetView, governor evidence and TermValue consumers retain their expected reservation error type.

## Consumers and required evidence

The actual interaction is XSD numeric ranges used by OWL cardinality preparation and its independent proof checker, plus the same numeric kernels used by SPARQL comparison, promotion, aggregate and cast paths. Public validation, C ABI and fresh native Python reasoning paths exercise the final proof service and trace identity. Main's retained SPARQL API and ownership implementation are preserved verbatim by the ordinary merge; #499 does not replace those carriers.

`main-508-resolution-versus-main.patch` records the integrated #499 delta at the numeric home and its actual regression. `main-508-integration-family-1.sh` records the affected strict/native/host and shared-helper qualification. Runtime, portable applicability, and normal signed merge publication remain NOT MET until their actual evidence is captured. The original full #499 gate and original main #508 acceptance remain historical evidence; neither is represented as a pass for this uncommitted combination.

## Current qualification

The first family ended 101 on one unnecessary qualification in the new private allocator parameter. The second ended 101 on the original-core-box/re-export collision above. Both failed logs remain failed records. The settled third family uses the same original 4-job/16GiB/no-swap lane and original assertions. Its complete affected all-target strict Clippy command has passed (1m42s); actual native/host execution is still running. This is not a full `make check` rerun. `main-508-portable-family-1.sh` records the next affected actual wasm32 numeric and optimized-package proof/checker family, currently NOT MET.

The settled third family has now passed every XSD/entail/validate native target, all59 selected SPARQL numeric cases, and all104 C ABI cases. The new unequal-scale range regression passed actual original allocation-count/peak/release assertions. The unchanged16-case proof/check byte corpus passed through the C ABI. Whole shared-helper hygiene and the86-domain/3-exception original hash registry also passed without an exemption. Fresh Python3.13 native binding compilation is the remaining live step; no complete family terminal is inferred before it finishes.

The original native family3 has now terminated0 PASS, session24547. The fresh Python3.13.12 native build passed and all70 consumer reasoning cases passed (0.39s). `main-508-native-family-result.md` binds the complete native/host/helper evidence. Actual portable family1 is now RUNNING in the same capped lane; source remains frozen. The parent source resolution judgment has no source finding (`reviews/main-508-resolution-review.md`), and its final runtime/portable disposition remains pending.

The original portable family1 has now terminated0 PASS, session50582. Actual
XSD3 and SPARQL2 WASM cases retain their original frozen digests and independent
answers. The fresh optimized public package passed its original post-link
suspend/run/poison and SIMD checks, followed by all43 actual public entailment,
proof-checker, byte-golden and typed-refusal cases with no skip or failure.
`main-508-portable-family-result.md` binds these terminal results to unchanged
staged tree `a222610de483e8c0fe0845ca362a4d5dbc5f50f7`. No working-tree delta or
unmerged entry remained, and whitespace checks passed. Final parent disposition,
normal signed merge hooks/push and fresh hosted/candidate acceptance remain
pending; no issue closure or historical full-gate reexecution is claimed.

The parent has read the complete native/portable terminal logs and updated the
same existing independent resolution review to **PASS** for this source/affected
integration. No new review panel or broad gate is introduced. Normal signed
synchronization hooks/commit/push now proceed on the unchanged tested tree;
fresh hosted/debt/candidate acceptance remains parent-owned and pending.

Normal signed synchronization commit and ordinary push are now each terminal0
PASS. Published head `b40c598a671c0baf3126b6930f0f165ffac66fc5` has valid Git
signature `G`, the original b719+a77 parents, and exactly tested tree
`a222610de483e8c0fe0845ca362a4d5dbc5f50f7`. Normal hooks ran, the working tree
is clean and ordinary `git ls-remote` reads back the exact remote issue head.
`main-508-sync-publication-result.md` records the actual logs and remaining
current-hosted/candidate/integration acceptance. PR527 was not recreated.
