# Review-Integration Verdict — Issue #294 (XML writers emit literals they cannot read back)

**Role:** review-integrator (DeepSeek Flash) — bookkeeping-only; no new opinions.
**Date:** 2026-09-14
**Plan reviewed:** `plan.md` (260 lines, timestamped 15:56)
**Disposition reviewed:** `assembler-disposition.md` (41 lines, timestamped 15:56)
**Reviews read (all 8):** 01-plan-reviewer, 02-plan-reviewer-2, 03-goals-reviewer,
04-goals-reviewer-2, 05-enhancement-auditor, 06-enhancement-auditor-2,
07-plan-adversary, 08-plan-adversary-2.

## Method

For every finding raised by the eight reviews (as consolidated in the disposition's
F1–F16 ledger plus the two standing declines) I located the corresponding text in
the revised plan and compared it against the finding, not against the plan's
"general quality". I also inverted the check: I walked each review's findings list
and confirmed every one is represented in the disposition ledger, so nothing was
silently dropped between review and disposition. Verdicts use the dispatch's three
buckets; "DECLINED" accepts a reason recorded in `assembler-disposition.md`
(per the dispatch wording "DECLINED (with reason in disposition)").

## Per-finding verdicts

| Finding (raised by) | Verdict | Evidence / reason |
|---|---|---|
| **F1** — `purrdf_iri::terminals::is_xml_char` does not exist and no task creates it (07 blocking, 08 blocking) | **ADDRESSED** | New **Task 1a** (`plan.md:72–92`) is titled "Spell the XML `Char` Terminal in `purrdf_iri::terminals`". `plan.md:78` gives the exact signature `pub const fn is_xml_char(char)  // XML 1.0 Fifth Edition §2.2 production [2] Char`; `plan.md:81` spells the range `#x9 \| #xA \| #xD \| [#x20-#xD7FF] \| [#xE000-#xFFFD] \| [#x10000-#x10FFFF]`, "via the existing `terminal!` macro, with the production cited … and its ranges asserted at compile time"; `plan.md:90` acceptance "exists with the production cited and ranges asserted at compile time"; `plan.md:91` requires `make terminal-hygiene`. |
| **F1a** — do NOT substitute `is_xml_name_char` (over-refusal of `#x9/#xA/#xD/#x20/U+0085/U+00A0` neighbours) (08) | **ADDRESSED** | `plan.md:81`: "**Do NOT substitute `is_xml_name_char`** — `NameChar` is a strict subset of `Char` and would refuse `#x9`/`#xA`/`#xD`/`#x20`/U+0085/U+00A0, i.e. exactly the valid neighbours R5 protects." Repeated in the Risk table at `plan.md:249`. |
| **F2** — `purrdf::serialize` / `purrdf::parse` do not exist (07 blocking, 08 blocking) | **ADDRESSED** | `plan.md:163`: tests use "`purrdf::serialize_dataset_to_format` / `purrdf::parse_dataset` (or `purrdf::dataset_from_bytes`) with `NativeRdfFormat::RdfXml` and `NativeRdfFormat::TriX`" and explicitly records "(`purrdf::serialize` and `purrdf::parse` do not exist.)". Also `plan.md:179`, `plan.md:240`. The corrected names match the disposition's cited sources. |
| **F3** — nonexistent path `crates/rdf-core/src/diagnostics.rs` (07 minor, 08) | **ADDRESSED** | `plan.md:150`: "`crates/rdf-core/src/diagnostic.rs` (singular — not `diagnostics.rs`) — any needed diagnostic plumbing". The parenthetical makes the correction explicit. |
| **F4** — internal contradiction: full `Char` check refuses U+FFFE/U+FFFF while the Enhancements table declines them (08; independently 01, 02, 04, 05, 06) | **ADDRESSED** | `plan.md:65` replaces the decline with "**INCORPORATED — not an enhancement**: The refusal boundary is the full XML 1.0 5e `Char` production, so U+FFFE/U+FFFF are refused inherently by the same check that refuses C0 controls; declining them would require a *narrower* predicate, contradicting Task 2." Contract R3 (`plan.md:52`), Task 2 (`plan.md:142`, `154`), Task 3 refusal matrix (`plan.md:168` "plus U+FFFE and U+FFFF"), and Risk table (`plan.md:250`) all state the single full-`Char` boundary. |
| **F5** — banned deferral language "Out of scope; Can be added later" (ETHOS §T) (01 blocking, 02 blocking, 04 blocking) | **ADDRESSED** | `plan.md:65` explicitly removes it: "(Draft's \"Out of scope; Can be added later\" wording removed — banned deferral language per ETHOS §T.)". A whole-file scan (`rg -i "can be added\|later\|out of scope\|defer\|future work\|phase 2\|for now\|TODO\|FIXME\|follow-up\|if feasible\|if needed"`) returns only the two lines that quote-and-negate the banned wording (`plan.md:65` and `plan.md:144` "explicit, not \"if feasible\""). No live deferral remains. |
| **F6** — module placement: `iri_escape.rs` (issue text, accepted by 07) vs `xml_escape.rs` (04 blocking, 05/06 category error) | **ADDRESSED** (resolved; reason recorded; operator-veto flagged) | `plan.md:64` records the adjudication and both positions, then takes the 04/05/06 side: "**ACCEPTED** (reversal of the draft's decline) … Followed 04/05/06: the issue's requirement is *one shared escape in a shared rdf-core home* … moving it into `iri_escape.rs` would be rework that makes the code worse." Task 1 (`plan.md:96–110`) targets `xml_escape.rs` with `pub mod xml_escape;`, and `plan.md:257` (Assembler Note) flags the single issue-text deviation for operator veto. |
| **F7** — broaden scope to all XML egress surfaces (projections, SVG, SRX) as one total boundary (05, 06) vs 07 "out of scope" | **ADDRESSED** (resolved; 05/06 side taken) | New **Task 1b** (`plan.md:121–136`) audits every XML-producing surface and delegates to `purrdf_core::xml_escape::{escape, push}` with correct `Context`; contract row **R9** (`plan.md:58`) names "codecs, projections, SVG, SRX". Acceptance `plan.md:134` requires no XML escaping outside the shared module. |
| **F8** — Task 5 hand-assembles gate commands, understating `make check` (04 blocking, 05, 06) | **ADDRESSED** | `plan.md:207–211` now runs `make check` (comment enumerates terminal-hygiene, build-profile-hygiene, check-no-features, check-generated) plus `make wasm`; mirrored in R8 (`plan.md:57`) and Success Criteria (`plan.md:242`). |
| **F9** — byte offset conditional / atomic semantics vague (05, 02, 04) | **ADDRESSED** | `plan.md:144`: "**Atomic semantics (explicit, not \"if feasible\"):** validate-then-write. The appending form (`push`) truncates the caller's buffer back to its original length on error; both codecs construct the complete document and only then append it in `RdfCodec::serialize_into`". `plan.md:146` makes the byte offset **required** ("cheap — `char_indices` provides it"); `plan.md:156` requires `serialize_into` with a nonempty caller buffer to leave it unchanged. |
| **F10** — test file location ambiguous ("or extend existing TriX harness") (01) | **ADDRESSED** | `plan.md:175`: "`crates/rdf/tests/xml_character_roundtrip.rs` — the RDF/XML + TriX suite (committed location; no alternative)"; SRX at `plan.md:176`. |
| **F11** — hedge "or adjacent module if architecture demands" in Task 1 (02) | **ADDRESSED** | The phrase is absent from the plan (whole-file search returns nothing). `plan.md:99` commits: "Consolidate XML literal escaping into the dedicated module `crates/rdf-core/src/xml_escape.rs`". |
| **F12** — refusal/valid-neighbour matrices need explicit boundary scalars (05, 06) | **ADDRESSED** | `plan.md:167` lists valid boundaries (`#x9`, `#xA`, `#xD`, U+007F, U+0085, U+00A0, U+D7FF/U+E000, U+FFFD, astral U+10000/U+1FFFE/U+10FFFF); `plan.md:168` enumerates the refusal matrix "U+0000–U+0008, U+000B, U+000C, U+000E–U+001F), plus U+FFFE and U+FFFF". |
| **F13** — exercise a production attribute path that can actually carry arbitrary whitespace (05, 06) | **ADDRESSED** | `plan.md:166` names the `rdf:parseType="Literal"` canonicalization path "whose embedded element attributes carry arbitrary whitespace — `xml:lang`/datatype IRIs cannot"; `plan.md:172` cites the WIP test `xml_literal_canonicalization_preserves_attribute_whitespace`. |
| *(un-numbered, folded into F13)* 05-5/06-4 ask for a call-site → `Context` ownership map | **ADDRESSED** | `plan.md:124` requires the audit to "delegate its character-data/attribute escaping … with the correct `Context`" and enumerates the known surfaces; `plan.md:134` acceptance forces repository-wide coverage. The tabular presentation was not adopted, but the substantive requirement (every emitter bound to the right context) is. Noted under Observations. |
| **F14** — Task 4 must distinguish frozen GTS vectors / generated projections / ordinary goldens (05, 06) | **ADDRESSED** | `plan.md:188–198` defines the three artifact classes with a stop-rule for frozen vectors ("must **never** be regenerated … stop; that is a defect, not a refresh"), `make metadata` only for `generated/`, and per-file before/after hash + reason for ordinary goldens. |
| **F15** — exhaustive all-scalar encoder-law sweep (05-6, 06-2) | **DECLINED (partial)** | Incorporated portions: Task 1a predicate spot-checks (`plan.md:92`) and Task 3 matrices (`plan.md:167–168`). The exhaustive 1.1M-scalar sweep is not in the plan; the disposition records the reason (`assembler-disposition.md:29`): "the `terminal!` macro's compile-time range assertions plus Task 1a's predicate spot-checks and Task 3's full C0+FFFE/FFFF integration refusal matrix carry the law; a separate 1.1M-scalar integration sweep is redundant with the macro's asserted tables." Change present + reason recorded → not MISSED. |
| **F16** — criterion benchmark for the escape path (04 non-blocking) | **DECLINED** | Recorded in the plan itself, `plan.md:68`: "Criterion benchmark for the escape path | **DECLINED** | Correctness fix with no performance claim; the escape remains a single linear scan, byte-determinism is gated by goldens, and no regression assertion is being made". Reason present. |
| XML 1.1 support / 1.0↔1.1 mode selection (05, 06, draft) | **DECLINED** | `plan.md:67`: "DECLINED | Project uses XML 1.0 5e; 1.1 is a dead spec, and a selectable version would create two incompatible fidelity domains, violating low/no-optionality". |
| Context-agnostic escape function (draft, confirmed by 07) | **DECLINED** | `plan.md:66`: "DECLINED | Must be context-aware (`Text` vs `Attribute`) per XML §3.3.3; a single context-free function would silently corrupt attribute whitespace". |
| Review 03 goals-reviewer | **n/a — no findings** | `03-goals-reviewer.md` returns "Verdict: PASS" with no findings; disposition row "No action required". Nothing to integrate. |

**Totals: 16 addressed, 4 declined (F15, F16, XML 1.1, context-agnostic), 0 missed.**

## Reasoning on the close calls (for the downstream reader)

- **F1 and F4 are the two that could have been MISSED, and both are plainly closed.**
  F1's failure mode was a plan that assumed a terminal predicate the committed tree
  does not contain, forcing either a hand-rolled range table (which
  `scripts/check-terminal-predicates.py` rejects) or an unpriced mid-implementation
  terminal authoring step. Task 1a prices and specifies exactly that step, including
  the compile-time range assertion and the hygiene gate, so R8 is now reachable.
  F4's failure mode was an implementer free to choose between a C0-only predicate and
  the full `Char` predicate; the plan collapses the choice to one boundary in six
  places (R3, Task 1, Task 2 description, Task 2 acceptance, Task 3 matrix, Risk
  table), so no ambiguity survives.

- **F5 was checked by negation, not by trust.** The disposition claims the deferral
  wording is gone; I ran a case-insensitive whole-file scan for the full ETHOS §T
  banned set. The only hits quote the removed wording for the record (`plan.md:65`)
  and the removed "if feasible" hedge (`plan.md:144`). This is the right kind of
  residue — a change-log entry, not a live deferral.

- **F15 is the one finding I could not call unambiguously ADDRESSED, and I did not
  force it.** The disposition itself labels it "PARTIALLY INCORPORATED". The
  incorporated part (boundary spot-checks + full integration refusal matrix) points
  at real plan text; the unincorporated part (exhaustive sweep) has a recorded,
  substantive reason (redundancy with the macro's compile-time-asserted tabulated
  ranges). Under the dispatch's rule that a disposition reason counts as a decline,
  this is DECLINED, not MISSED. I flag it here rather than silently promoting it,
  because a future reader may disagree with the redundancy claim.

- **F6/F7 are adjudicated contradictions, which is why they are not "declined".**
  In each, two reviewer camps directly opposed each other, and the plan picked a
  side and recorded both positions plus the reason. Recording both positions and an
  operator-veto hook (F6, `plan.md:257`) is the strongest available form of
  resolution; a silent pick would have been the failure.

- **What I deliberately did not flag.** I did not re-open (a) the correctness of the
  full-`Char` boundary — that is the reviewers' judgement, already adjudicated; (b)
  the `xml_escape.rs` vs `iri_escape.rs` choice on its merits — my job is whether
  the finding was answered, not whether the answer is architecturally right; (c) the
  Task numbering oddity (`Task 1a` precedes `Task 1` in document order) — not a
  review finding, so out of scope; or (d) the SOTA breadth of the enhancement
  auditors' transformation programme — the plan's Task 1b/R9 is the disposition's
  answer to it, and re-litigating scope is a reviewer's job, not mine.

## Observations (out of scope for verdicts; recorded once, not actioned)

1. **Operator-veto item, not a defect:** the module home deviates from the literal
   issue text (`xml_escape.rs` vs the issue-named `iri_escape.rs`). `plan.md:257`
   already flags this; whoever presents the plan should surface it explicitly.
2. **Cosmetic, no verdict:** `Task 1a` is ordered before `Task 1`. Harmless, but a
   reader skimming the task list may stumble.
3. **Minor wording drift, already fenced:** contract row R3 at `plan.md:52` still
   leads with "Refuse C0 scalars outside XML `Char`" even though its acceptance text
   and Task 2 require the full `Char` production. The parenthetical in the same cell
   makes the superset explicit, so this is not ambiguous enough to be a MISSED.
4. **F15 residual:** if a later stage wants the exhaustive sweep, it should say so
   affirmatively rather than treating the disposition's redundancy reason as settled
   — the reason is plausible but unverified in this pass (I did not read the
   `terminal!` macro implementation).

## Final verdict

**PASS — all findings are ADDRESSED or DECLINED with a recorded reason. 0 MISSED.**
The plan is ready to present. It carries one flagged operator-veto decision (module
home) and one partial decline (F15 exhaustive sweep) that the operator may wish to
revisit, but neither is an unintegrated finding.
