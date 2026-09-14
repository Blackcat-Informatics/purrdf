## Plan Review

**Plan:** `plan.md` (issue #294 — XML writers emit literals they cannot read back)
**Reviewed:** 2026-09-14
**Reviewer:** plan-reviewer-2 (Z-AI model)
**Rubric:** `~/.config/opencode/eval/ETHOS-FULL.md` + SOLID principles

---

## SOLID Assessment

| Principle | Status | Confidence | Notes |
|-----------|--------|------------|-------|
| SRP | PASS | HIGH | Escape logic consolidated to one module; validation, testing, audit, verification, and PR each have a single, bounded responsibility. No god-class potential. |
| OCP | PASS | HIGH | Extends existing `iri_escape.rs` rather than fragmenting into new modules. Context-aware API (Text vs Attribute enum) creates extension points without editing stable writer code. Declined a new `xml_escape.rs` module in favor of augmenting the existing egress home. |
| LSP | PASS | HIGH | Two byte-identical `escape_xml` functions are replaced by one shared implementation; substitutability is preserved because the replacement honors the same contract (escape `&`, `<`, `>`, `"` plus new `#xD` handling). No new trait hierarchies introduced, so LSP concerns are minimal. |
| ISP | PASS | HIGH | Splits context-sensitive escaping into `escape_xml_text` and `escape_xml_attr` rather than a monolithic function with a boolean or string flag. Clients depend only on the interface they need. |
| DIP | PASS | HIGH | Writers depend on the shared abstraction in `rdf-core` rather than their own concrete implementations. Validation delegates to `purrdf_iri::terminals::is_xml_char` — the existing, spelled-once predicate — rather than a hand-rolled heuristic. |

#### SOLID Details

**SRP Analysis:**
Task 1 handles escaping consolidation, Task 2 handles invalid-character refusal, Task 3 handles integration tests, Task 4 handles golden audit, Task 5 handles gate verification, and Task 6 handles PR. Each task has exactly one reason to change. The shared escape module is concerned only with XML-safe string serialization; diagnostic construction lives in `diagnostics.rs`.

**OCP Analysis:**
The plan correctly identifies that a new `xml_escape.rs` module would be premature fragmentation. By adding `escape_xml_text`/`escape_xml_attr` to `iri_escape.rs`, the existing escape infrastructure grows by addition, not by rewriting the two writers. The enum context parameter (`Text` vs `Attribute`) is an extension point: if XML 1.1 or CDATA context were ever needed (not planned), the enum variant can be added without touching the call sites that use `Text`.

**LSP Analysis:**
No new subtyping relationships are introduced. The plan replaces two identical private functions with one shared private function. The behavioral contract ("output is safe for XML 1.0 character data / attribute values") is preserved and strengthened (adds `#xD` handling and invalid-char refusal). LSP is satisfied trivially.

**ISP Analysis:**
The plan explicitly declined a "context-agnostic escape function" because XML §3.3.3 mandates different rules for text and attribute contexts. Splitting into two named functions (`escape_xml_text`, `escape_xml_attr`) is the correct ISP response: a caller writing element text does not need to know about attribute normalization rules.

**DIP Analysis:**
Both `rdfxml.rs` and `trix.rs` currently depend on their own local `escape_xml` concrete implementations. The plan inverts this: they will depend on the shared abstraction in `rdf-core`. The validation step further depends on `purrdf_iri::terminals::is_xml_char` (the existing abstraction) rather than duplicating the `Char` production locally.

---

## ETHOS Compliance

| Section | Status | Notes |
|---------|--------|-------|
| Fail Fast (§2-9 / §H) | COMPLIANT | Invalid chars (U+0000) produce hard errors with named scalars, non-zero exit, atomic semantics (no partial output). No conditional imports, no nullable escape hatches. |
| Maximum Utility (§5 / §O) | COMPLIANT | Default behavior is strict and comprehensive. The context-aware API is spec-mandated, not optional. No `strict=true` or `validate=true` flags. |
| One Path (§19 / §O) | COMPLIANT | Two duplicate `escape_xml` functions collapse to one shared implementation. No modal boolean switches. Context is an enum variant, not a behavior flag that creates divergent code paths. |
| No Shortcuts (§21 / §T) | **VIOLATION** | Line 49 contains deferral language: `"Can be added later."` — banned under ETHOS §T ("Banned resolutions: 'deferred', 'follow-up', 'future work', 'phase 2', 'for now', TODO/FIXME markers, stub implementations, partial delivery reported as complete"). Additionally, `"Out of scope"` on the same line rationalizes leaving a known defect unaddressed. |

#### ETHOS Details

**Evidence over Assertion (§E):**
The plan cites specific prior commits (`84abf779`, `7a4d9a7b`), specific W3C productions (XML 1.0 §2.11, §3.3.3), and an existing internal predicate (`purrdf_iri::terminals::is_xml_char`). The prior-art section explains *why* previous fixes did not cover this surface. This is evidence-based planning, not assertion.

**Total Ownership (§T):**
The plan correctly owns the full scope of the issue: shared implementation, invalid-char refusal, round-trip tests, golden audit, gate verification, and PR. However, line 49 contains:

> `| Refuse U+FFFE/U+FFFF (also excluded by Char) | **DECLINED** | Out of scope; issue focuses on C0 controls. Can be added later. |`

**This is a clear ETHOS §T violation.** `"Can be added later"` is exactly the deferral language banned by ETHOS. `"Out of scope"` is a rationalized shortcut: U+FFFE and U+FFFF are *also* excluded by the XML `Char` production that the plan already proposes to enforce. Refusing them is not a separate enhancement — it is the *same* validation rule applied consistently. Declining to apply a rule the plan already implements, on the grounds that it is "out of scope," is leaving a known defect in place while calling the work complete. ETHOS §T: "If something is genuinely impossible or ill-conceived, STOP and say so explicitly. Descoping is the requester's decision, never yours."

**Hard Failures, Never Silent Degradation (§H):**
The plan correctly proposes hard failures: U+0000 produces a named error, non-zero exit, no partial output. It uses the existing `is_xml_char` predicate from `purrdf_iri::terminals` (the single spelled-once production) rather than a heuristic. Validation is unconditional. The "atomic semantics" requirement (no output before error) is correct. PASS.

**One Path, Maximum Utility (§O):**
The plan consolidates two identical functions into one shared implementation — exactly what §O requires. The context enum (`Text` vs `Attribute`) does not create modal behavior switches in the sense ETHOS condemns; XML 1.0 §3.3.3 *requires* different escaping rules for attributes vs text, so the split is spec-mandated, not optional. The plan correctly declines a context-agnostic function. Maximum utility is served by escaping `#xD` and refusing invalid chars by default. PASS.

**Simplicity and Structure (§S):**
The plan is structurally sound. Contracts are implied (shared escape API with `Result` return). YAGNI is respected: no XML 1.1 support, no new module, no speculative abstractions. The plan uses the standard library and existing infrastructure (terminals, diagnostics) rather than hand-rolled character tables.

---

## Optionality to Remove

| Location | Current | Recommendation |
|----------|---------|----------------|
| Line 49, Enhancement table | `Refuse U+FFFE/U+FFFF … DECLINED … Out of scope … Can be added later.` | **Remove the row entirely.** U+FFFE and U+FFFF are excluded by the same XML `Char` production the plan already enforces. The shared `is_xml_char` check will naturally reject them. There is no additional work; the plan already covers them. Calling this "out of scope" is incorrect. |

#### Specific Optionality Issues

1. **U+FFFE/U+FFFF deferred to "later"**
   - Current: `"Can be added later."`
   - Problem: This is explicit deferral language, banned by ETHOS §T. U+FFFE/U+FFFF are excluded by XML `Char` — the same production the plan validates against. The implementer does not need to add special handling; `is_xml_char` already returns `false` for them. This is not an enhancement; it is the natural consequence of the validation already planned.
   - Fix: Delete the row. If the implementer wishes to document that these scalars are rejected, add them to the test matrix in Task 3 (e.g., `"U+FFFE produces error"`) as a valid-neighbour/invalid-neighbour test case.

---

## Plan Quality

| Aspect | Status | Notes |
|--------|--------|-------|
| Acceptance criteria | CLEAR | Every task has a bulleted acceptance list. Success Criteria checklist (10 items) maps back to issue requirements. |
| Risk assessment | PRESENT | Four risks identified with concrete mitigations (over-refusal, partial output, wrong Char predicate, text/attribute confusion). |
| Phase breakdown | GOOD | 6 tasks in logical order: consolidate → validate → test → audit → verify → PR. Task dependencies are implicit and sensible. |
| Verification steps | DEFINED | Task 5 lists the exact gate commands (`cargo fmt --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`, `make wasm`). |
| PR task present | YES | Task 6: PR Creation, with required body contents listed. |
| Completeness Contract | COMPLETE | All 8 requirements (R1–R8) are mapped to tasks (T1–T5). No orphan requirements. |
| Task numbering | CORRECT | Starts at Task 1 (setup is `stagectl open`'s job). No phantom Task 0. |

---

## Improvements Suggested

1. **Delete line 49 (U+FFFE/U+FFFF enhancement row).** It contains banned deferral language (`"Can be added later"`) and mischaracterizes a natural consequence of the planned validation as an out-of-scope enhancement. The plan already enforces XML `Char`; U+FFFE/U+FFFF are naturally rejected.
2. **Add U+FFFE/U+FFFF to the invalid-scalar test matrix in Task 3.** Since `is_xml_char` covers them, the round-trip tests should prove it. This turns the "declined enhancement" into verified behavior at zero cost.
3. **Clarify atomic error semantics in Task 2.** The acceptance says `"No output written before error (atomic semantics)"` but does not specify the mechanism. Recommend: validate the entire literal string before writing any bytes, or use a buffered writer that is flushed only on success. A one-sentence implementation hint would help the implementer.
4. **Task 1 mentions `"or adjacent module if architecture demands"`.** This is a minor hedge. The standing constraints say extend `iri_escape.rs`; I recommend removing the hedge and committing to `iri_escape.rs` unless the implementer discovers a real architectural blocker during work.

---

## Overall Assessment

**Status:** **REJECTED**

**Summary:** The plan is architecturally sound, SOLID-compliant, and correctly scoped for the issue. The Completeness Contract maps all 8 requirements to tasks, the task numbering starts correctly at 1, and the PR task is present. However, the plan contains explicit deferral language on line 49 — `"Can be added later"` — which is banned by ETHOS §T ("Banned resolutions: 'deferred', 'follow-up', 'future work', 'phase 2', 'for now', TODO/FIXME markers, stub implementations, partial delivery reported as complete"). The same line also uses `"Out of scope"` to rationalize leaving a known defect unaddressed. In this specific case, the "declined enhancement" (refusing U+FFFE/U+FFFF) is not actually additional work: the plan already proposes to validate against XML `Char` via `is_xml_char`, and that predicate naturally rejects U+FFFE/U+FFFF. The row is therefore both ethically non-compliant and technically incorrect. Once line 49 is removed and U+FFFE/U+FFFF are added to the test matrix as natural invalid cases, the plan is approvable.

**Blocking Issues:**
1. **Line 49 contains banned deferral language:** `| Refuse U+FFFE/U+FFFF (also excluded by Char) | **DECLINED** | Out of scope; issue focuses on C0 controls. Can be added later. |`
   - `"Can be added later"` is explicit deferral language, banned by ETHOS §T.
   - `"Out of scope"` rationalizes not applying a validation rule the plan already implements.
   - **Fix required:** Remove the row entirely; add U+FFFE/U+FFFF to Task 3's invalid-scalar test cases.

**Recommendations:**
1. Add U+FFFE and U+FFFF as invalid-scalar test cases in Task 3 (they are naturally rejected by `is_xml_char`).
2. Remove the hedge `"or adjacent module if architecture demands"` from Task 1 and commit to `iri_escape.rs`.
3. Add a one-sentence implementation hint for atomic error semantics in Task 2 (validate-then-write or buffered write).
