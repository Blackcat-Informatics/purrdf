<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #470 current full-contract material review

## October 14 current-head applicability addendum

The verdict below records the earlier `538eb2f71` state and must not be read
as a review of today's clean, normally pushed `50ae94847`. The previously
dirty #531 evaluator experiments have been transported to the dedicated
main-based #531 delivery; this #470 worktree is clean. The previously stale
bounded-comparison helper exemption was removed in `50ae94847` and the
unchanged helper gate passed (81 enforced jobs); the prior complete `make
check` remains terminal 2 because its suffix never ran on this corrected
head. I re-read the current issue contract, plan, validation and source/host
mapping; no further #470 numeric API, division, temporal, profile or
documentation implementation gap is demonstrated. The distinct #443 kernel
requirements remain open in their own survivor.

The selected XSD/CLI/C/Python/WASM numeric, native/wasm determinism,
governor, conformance, metadata/i18n and document receipts keep their stated
source-bound scopes. They do not qualify a package with the forthcoming #531
evaluator integration. The old shared-driver full npm run failed exactly
three original #531 floors; it is not a waiver or a passing #470 gate. The
remaining reviewable requirements are ordinary #531 integration, affected
installed host rebuilds and numeric cells, passing unchanged optimized npm
and repository gates, then integrated-head material review, hosted PR checks,
protected merge and closure verification. This addendum is a read-only
source/evidence assessment; no new build or host test was run for it.

**VERDICT: BLOCKED for complete #470 delivery.** Read-only review of the live issue body, current four-task Stage plan, committed/pushed HEAD `538eb2f71`, production/API/host source, and retained validation. I ran no builds or tests and changed no production source. The dirty `crates/sparql-eval/src/binop.rs` and `expr.rs` diffs are separate, uncommitted #531 stack experiments; they cannot qualify or be silently included in a clean #470 delivery.

## Contract implementation observed

I found **no specific missing #470 production behavior** in the inspected committed source. `XsdValue` uses `exact::Integer` and `exact::Decimal`; the old bounded `numeric::Decimal` and conversion bridge remain crate-private implementation details, not public alternatives. `DivisionPolicy::xsd_default()` is `ExactOrScale { scale: 18, rounding: HalfEven }`; `exact`, bare `N` (toward zero), and `N:ROUNDING` parse and propagate through the evaluator and the CLI, Python, C, and sync/async WASM entry points. Temporal instant and duration seconds use `exact::Decimal`, including the evaluator's `SECONDS` result. The issue's governor version/vector, Breaking guidance, English book and zh-Hans requirements have source and focused receipt coverage. The removed #443-only Toom/parser changes are preserved in the donor and are **not** #470 acceptance criteria.

The accepted host cells exist in current test source: >18-place terminating `1/1048576`, repeating `2/3`, true half-even `3/8` at scale 2, explicit exact/bare-scale/named rounding, and 21-place temporal `SECONDS`. Retained final-source receipts record XSD full crate/strict checks, governor v18 replay 15 passed with its regeneration test ignored, CLI 8/8, C smoke 6/6, installed Python wrapper 4,223 passed with 1 skip/4 xfail plus CLI 1/1, and optimized WASM selected numeric sync/async 13/13. The full conformance regeneration was GREEN (16,362 pass/16 ledgered/0 fail); the measured two-row document correction was committed at `838667cab`, and metadata/i18n/doc claims have passing receipts. These observations do not constitute a whole-workspace or portable pass.

## Literal acceptance still missing or failed

1. **Required full repository gate: FAILED on last complete attempt, then not rerun on corrected HEAD.** `make check` on `838667cab` stopped in all-target strict Clippy with retrieval cursor and geo exact-integer fixture errors. Their focused corrections passed and were normally committed at `538eb2f71`, but that is not a successful `make check` or its later stages. Run the unchanged gate on a settled source and resolve any actual failures.
2. **Full optimized Node package check: FAILED.** The numeric package's `npm run check` passed the deep async case but exited 1 on three unchanged accepted-depth floors (sync/job 134/134 < 181, LATERAL 234 < 281, NOT EXISTS 133 < 180). A fresh default-directory artifact with the LATERAL helper still failed all three; a separate EXISTS probe reduced only constant bytes and was rejected. The [bounded classification](issue-531-versus-470-stack-floor-classification.md) establishes the same failure mode on PR #535's hosted, main-based package before #470, while any small #470 depth delta remains unknown. This is #531 production debt, not proof of a missing numeric policy. It remains a **failed Task 3 package gate**, never a passing #470 result or a waived check. The current dirty definition-finisher experiment in `expr.rs` has no final optimized-floor receipt.
3. **Final-source portable/release acceptance: OPEN.** The Stage plan calls for native/portable numeric determinism and applicable release WASM/portable checks. The recorded native XSD determinism and optimized Node policy tests are positive but do not establish that entire portable matrix on settled #470 source. Current exact source/artifact identities and final cross-host lexical, typed-error and profile readbacks should be reconciled in one final acceptance record; existing package receipts may be reused when their source remains applicable.
4. **Integration: OPEN.** The current worktree includes unqualified #531 modifications. Isolate or restore those before declaring a #470 final head; reconcile main, complete final-source review and required gates, then PR, hosted checks, `ghprsq` merge, issue closure and mirror readback. No PR or #470 closure is established.

There is no basis here to invent a #443 performance or exclusive-timing gate for #470. Conversely, selected green host cells, a hooked checkpoint and a separate owner for the Node floors do not make the failed mandatory package/full-workspace gates successful. The actionable next step is to settle the source boundary, run the missing final gates, and classify their exact outcomes rather than another implementation campaign without a demonstrated #470 defect.
