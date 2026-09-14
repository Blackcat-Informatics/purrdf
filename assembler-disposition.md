# Assembler Disposition — Issue #294 (XML writers emit literals they cannot read back)

**Assembled:** 2026-09-14
**Inputs:** draft plan + prior-art brief + 8 reviews (01 plan-reviewer, 02 plan-reviewer-2, 03 goals-reviewer, 04 goals-reviewer-2, 05 enhancement-auditor, 06 enhancement-auditor-2, 07 plan-adversary, 08 plan-adversary-2)

## Factual axis discovered during assembly

The panel split on workspace state: 05/06 examined the worktree's **uncommitted WIP** (which already contains `is_xml_char` in `terminals.rs`, `purrdf_core::xml_escape`, codec/projection/SVG/SRX migrations, and round-trip tests); 07/08 verified against the **committed** tree where none of that exists (both adversaries' greps are correct for `main`; reviewer 08's claim that the worktree shares main's working tree is wrong). The deficiency ledger is empty, so the WIP is adoptable work-in-progress, not flagged defective output. The revised plan is written against the committed baseline and directs the implementer to adopt, verify, and complete the WIP.

## Per-finding disposition

| # | Finding | Raised by | Disposition |
|---|---------|-----------|-------------|
| F1 | `purrdf_iri::terminals::is_xml_char` does not exist (committed tree); no task creates it; terminal ring-fence makes hand-rolling a gate failure | 07 (blocking), 08 (blocking) | **INCORPORATED** — new Task 1a spells `Char` once in `purrdf_iri::terminals` (XML 1.0 5e §2.2 `[2]` cited, ranges asserted at compile time, `terminal!` macro pattern, `make terminal-hygiene` acceptance); notes WIP already contains a conforming block to adopt |
| F1a | Substituting `is_xml_name_char` would over-refuse `#x9/#xA/#xD/#x20/U+0085/U+00A0` (the R5 neighbours) | 08 | **INCORPORATED** — explicit "do NOT substitute `is_xml_name_char`" warning in Task 1a and Risk table |
| F2 | `purrdf::serialize` / `purrdf::parse` do not exist | 07 (blocking), 08 (blocking) | **INCORPORATED** — Task 3 now names `purrdf::serialize_dataset_to_format` (writer) and `purrdf::parse_dataset` / `purrdf::dataset_from_bytes` (readers) with `NativeRdfFormat::{RdfXml, TriX}`; verified against `crates/rdf/src/native_codecs/{serialize.rs:480, parse.rs:137}`, `crates/rdf/src/dataset_io.rs:27` |
| F3 | `crates/rdf-core/src/diagnostics.rs` does not exist | 07 (minor), 08 | **INCORPORATED** — corrected to `crates/rdf-core/src/diagnostic.rs` (singular) |
| F4 | Internal contradiction: Task 2 checks full `Char` (refuses U+FFFE/U+FFFF for free) while the Enhancements table declines them | 08; independently 01, 02, 04, 05, 06 | **INCORPORATED** — refusal boundary stated once as the full XML 1.0 5e `Char` production (superset of the issue's C0 scope); declined-row replaced by an INCORPORATED note; U+FFFE/U+FFFF added to Task 3's refusal matrix |
| F5 | "Out of scope; Can be added later" — banned deferral language (ETHOS §T) | 01 (blocking), 02 (blocking), 04 (blocking) | **INCORPORATED** — language removed with the row's replacement; no deferral wording remains anywhere in the plan |
| F6 | Module placement: `iri_escape.rs` (issue text, draft plan; accepted by 07) vs dedicated `xml_escape.rs` (04 blocking as .goals naming compromise; 05/06 "category error" — IRIREF transcription ≠ XML character-data egress) | 04, 05, 06 vs 07 + issue text | **CONTRADICTED** — took the 04/05/06 side (`purrdf_core::xml_escape`). Reasons: (a) architectural — different grammar, different normalization law, one module per grammar boundary; (b) the issue's requirement is one shared escape in a shared rdf-core home; the filename was written when `iri_escape.rs` was the only egress home; (c) the WIP already implements `xml_escape.rs` and forcing `iri_escape.rs` would be rework making the code worse; (d) resolves 04's .goals blocker. Both positions named in the plan's Enhancements table; flagged for operator veto in the plan's Assembler Note |
| F7 | Broaden scope to all XML egress surfaces (projections, SVG, SRX) as one total boundary | 05 (transformation), 06 (transformation) vs 07 (explicitly out of scope) | **CONTRADICTED** — took the 05/06 side. Reasons: the prior-art brief documents the *third surface* of the same duplicated-escape defect class (predicates `84abf779` → IRIREF `7a4d9a7b` → XML literals), so the fix is scoped to the layer, not the point; and the WIP already migrates all three surfaces — excluding them would orphan completed, unflagged work. New Task 1b + contract row R9; issue's named surfaces remain the acceptance core (R1–R8) |
| F8 | Task 5 hand-assembles gate commands, understating `make check` | 04 (blocking), 05, 06 | **INCORPORATED** — Task 5 now runs `make check` (includes terminal-hygiene, build-profile-hygiene, check-no-features, check-generated) plus `make wasm` |
| F9 | Byte offset conditional ("if feasible"); atomic semantics vague ("use buffer if needed") | 05 (robustness), 02, 04 | **INCORPORATED** — byte offset required (cheap via `char_indices`); atomicity stated explicitly: `push` truncates to caller's original length; whole-document construction at the codec boundary; `serialize_into` with nonempty buffer tested |
| F10 | Test file location ambiguous ("or extend existing TriX harness") | 01 | **INCORPORATED** — committed to `crates/rdf/tests/xml_character_roundtrip.rs` (+ `crates/sparql-results/tests/xml_characters.rs` for SRX); matches WIP |
| F11 | "or adjacent module if architecture demands" hedge in Task 1 | 02 | **INCORPORATED** — removed; placement committed to `xml_escape.rs` (moot given F6) |
| F12 | Refusal/valid-neighbour matrices need explicit boundary scalars (U+0008/U+0009, U+000B/U+000C, U+000E–U+001F, U+007F, U+D7FF/U+E000, U+FFFD/U+FFFE/U+FFFF, U+10000/U+10FFFF) | 05, 06 | **INCORPORATED** — Task 3 matrix enumerates them; WIP test already covers all C0 + U+FFFE/U+FFFF refusal and the valid boundaries |
| F13 | Exercise a production attribute path that can actually carry arbitrary whitespace (XMLLiteral embedded attributes, not `xml:lang`) | 05, 06 | **INCORPORATED** — Task 3 names the `rdf:parseType="Literal"` canonicalization path; WIP's `xml_literal_canonicalization_preserves_attribute_whitespace` test cited |
| F14 | Task 4 must distinguish frozen GTS vectors (never regenerate) from generated projections and ordinary goldens | 05, 06 | **INCORPORATED** — Task 4 rewritten with the three artifact classes and a stop-rule for frozen vectors |
| F15 | Exhaustive all-scalar encoder-law sweep at the shared primitive | 05, 06 | **PARTIALLY INCORPORATED** — the `terminal!` macro's compile-time range assertions plus Task 1a's predicate spot-checks and Task 3's full C0+FFFE/FFFF integration refusal matrix carry the law; a separate 1.1M-scalar integration sweep is redundant with the macro's asserted tables |
| F16 | Criterion benchmark for the escape path | 04 (non-blocking, "not mandatory") | **DECLINED** — correctness fix with no performance claim; escape remains a single linear scan; byte-determinism gated by goldens. Recorded in the plan's Enhancements table |
| — | XML 1.1 support / 1.0↔1.1 mode selection | declined in draft; 05/06 independently rejected | **DECLINED** (unchanged) — dead spec; a mode would create two fidelity domains |
| — | Context-agnostic escape function | declined in draft; confirmed by 07 | **DECLINED** (unchanged) — §3.3.3 mandates context awareness |
| — | 03 goals-reviewer | PASS, no findings | No action required |

## Open Decisions

None escalated. The single deviation from literal issue text (module home `xml_escape.rs` vs the issue-named `iri_escape.rs`, F6) was adjudicated as technical, is recorded with both positions in the plan, and is flagged for operator veto in the plan's `## Assembler Note`.

## Prior-art scope effect

The brief documents this as the third surface of the duplicated-escape defect class (predicates → IRIREF → XML literals); per the recurrence discipline the plan widens from the two named codecs to the whole XML egress layer (Task 1b: projections, SVG, SRX), which also matches the uncommitted WIP already present in the worktree — rather than a point fix on `rdfxml.rs`/`trix.rs` alone.
