## Plan Review

**Plan:** plan.md
**Reviewed:** Mon Sep 14 2026

### SOLID Assessment

| Principle | Status | Confidence | Notes |
|-----------|--------|------------|-------|
| SRP | PASS | HIGH | Responsibilities cleanly separated: escaping logic consolidated, error handling distinct, tests separate |
| OCP | PASS | HIGH | Extends existing `iri_escape.rs` rather than modifying stable interfaces; new functions added without changing existing ones |
| LSP | PASS | MED | Error contracts are well-defined; no protocol violations apparent |
| ISP | PASS | HIGH | Separate `escape_xml_text` and `escape_xml_attr` functions provide granular interfaces |
| DIP | PASS | HIGH | Uses existing `purrdf_iri::terminals::is_xml_char` abstraction rather than hand-rolling validation |

#### SOLID Details

**SRP Analysis:**
The plan correctly consolidates duplicate `escape_xml` logic from two writers into one shared location. Each component has one reason to change: escape logic changes → `iri_escape.rs`; writer formatting changes → `rdfxml.rs`/`trix.rs`; test coverage → integration test file. The context-aware split (Text vs Attribute) is spec-mandated, not a responsibility split — both are still escaping.

**OCP Analysis:**
The plan adds new functions to an existing utility module rather than editing the writers' internal logic. The writers will call into the shared utility, extending behavior without modifying the escape algorithm's stable core. The "DECLINED" enhancement to create a new `xml_escape.rs` module respects OCP by reusing the existing egress home.

**LSP Analysis:**
No abstract protocols or inheritance hierarchies are involved. The `Result<(), XmlEscapeError>` contract is standard and substitutable. The planned implementations will satisfy the contract of returning an error for invalid input rather than panicking or producing malformed output.

**ISP Analysis:**
The context-aware API (Text vs Attribute) is split into two functions. This prevents clients from depending on more than they need — an attribute serializer does not need text-context logic and vice versa. This is appropriate ISP.

**DIP Analysis:**
The plan depends on the existing `purrdf_iri::terminals::is_xml_char` predicate (an abstraction) rather than writing a new `Char` check inline. This is correct per AGENTS.md's terminal ring-fence: "Every W3C terminal is spelled once, in `purrdf_iri::terminals`". The risk mitigation table explicitly says "Use `purrdf_iri::terminals::is_xml_char`, not heuristic" — this is DIP in action.

### ETHOS Compliance

| Section | Status | Notes |
|---------|--------|-------|
| Fail Fast (§2-9) | COMPLIANT | No conditional imports, no optional types, validation returns error instead of silent degradation |
| Maximum Utility (§5) | CONCERN | U+FFFE/U+FFFF handling contradicts scope; see details |
| One Path (§19) | COMPLIANT | Context-aware API is spec-required, not modal boolean; single canonical escape path |
| No Shortcuts (§21) | VIOLATION | Deferral language present |

#### ETHOS Details

**Fail Fast Analysis:**
The plan correctly implements hard failures: invalid XML characters produce errors rather than silently writing malformed output. "Validate-then-write; use buffer if needed" ensures atomic semantics (no partial output on error). No "if available" checks, no fallback paths, no nullable escape hatches. This is well-aligned with §H.

**Maximum Utility Analysis:**
The plan intends to use `purrdf_iri::terminals::is_xml_char` which covers the complete XML `Char` production. However, the enhancements table (line 49) explicitly declines to refuse U+FFFE/U+FFFF, which `is_xml_char` already covers. This creates a contradiction: either the implementation will use `is_xml_char` and naturally cover these characters (making the "declined enhancement" inaccurate), or it will write a narrower check specifically for C0 controls (contradicting the risk mitigation table). Properly implementing `is_xml_char` means maximum utility is achieved; the plan text should reflect this reality rather than disclaim it. Using the full `Char` predicate is the maximal behavior; pretending the scope is narrower is the shortcut.

**One Path Analysis:**
The context parameter (Text vs Attribute) is required by XML §3.3.3 and does not create two divergent code paths in the harmful sense — it's a single escape function with spec-compliant contextual behavior. Both RDF/XML and TriX writers use the same shared implementation. This is acceptable per ETHOS §O exception for spec-required differentiation and per the "one shared egress home" principle from prior art.

**No Shortcuts Analysis:**
VIOLATION. Line 49 contains explicit deferral language banned by ETHOS §T: "Banned resolutions: 'deferred', 'follow-up', 'future work', 'phase 2', 'for now', TODO/FIXME markers, stub implementations, partial delivery reported as complete." The review mandate explicitly lists "out of scope" and "later" language as grounds for rejection. The line "Out of scope; issue focuses on C0 controls. Can be added later." is a rationalized shortcut: it frames the correct behavior (rejecting all characters excluded by XML `Char`) as an optional future enhancement rather than the natural consequence of using the correct predicate.

### Optionality to Remove

| Location | Current | Recommendation |
|----------|---------|----------------|
| Enhancement table row | "Out of scope; Can be added later" | Remove row or rephrase: "Already covered by `is_xml_char` implementation; no additional work required" |

#### Specific Optionality Issues

1. **U+FFFE/U+FFFF "Enhancement" (line 49)**
   - Current: "Refuse U+FFFE/U+FFFF (also excluded by `Char`) | **DECLINED** | Out of scope; issue focuses on C0 controls. Can be added later."
   - Problem: Explicit deferral language ("Out of scope", "Can be added later") violates ETHOS §T and the review mandate. Furthermore, if `is_xml_char` is used per the risk mitigation table and Task 2, U+FFFE/U+FFFF ARE in scope automatically — they are excluded by XML `Char` just like C0 controls. To NOT handle them would require writing a MORE SPECIFIC check, which contradicts the stated intent to use the existing terminal predicate. This is backwards: the plan should embrace the full coverage of the correct predicate.
   - Fix: Remove the row entirely, or rephrase to acknowledge that `is_xml_char` covers all characters excluded by XML `Char` including U+FFFE/U+FFFF, so no separate enhancement is needed.

### Plan Quality

| Aspect | Status | Notes |
|--------|--------|-------|
| Acceptance criteria | CLEAR | Success Criteria checklist covers all requirements with specific checks |
| Risk assessment | PRESENT | Risk Mitigation table addresses 4 specific, well-identified risks |
| Phase breakdown | GOOD | 6 well-scoped tasks with clear boundaries and acceptance criteria |
| Verification steps | DEFINED | T5 lists explicit commands (fmt, clippy, test, wasm); T3 specifies writer→reader→graph isomorphism pattern |

### Improvements Suggested

1. Remove or rephrase the U+FFFE/U+FFFF enhancement row to eliminate deferral language and accurately reflect that `is_xml_char` covers all characters excluded by XML `Char`. The fix for C0 controls IS the fix for U+FFFE/U+FFFF when the correct predicate is used.
2. Clarify Task 2 description: state that `is_xml_char` validates the full XML `Char` production, which naturally covers both C0 controls and U+FFFE/U+FFFF, avoiding the false impression of a narrower scope.
3. Specify exact test file location rather than "Or extend existing TriX round-trip harness" to reduce ambiguity.

### Overall Assessment

**Status:** NEEDS REVISION

**Summary:** The plan is well-structured with clear tasks, good risk assessment, and solid SOLID compliance. The escape logic consolidation, error handling design, and round-trip verification approach are all sound. However, line 49 contains banned deferral language ("Out of scope", "Can be added later") which violates ETHOS §T "Work Is NOW — No Deferrals." Per the review mandate, any deferral language requires rejection. Additionally, the U+FFFE/U+FFFF row contains a substantive contradiction with Task 2 and the risk mitigation table: the plan intends to use `purrdf_iri::terminals::is_xml_char` (which covers the full XML `Char` production, including U+FFFE/U+FFFF), yet explicitly declines to handle those characters as "out of scope" and deferred. Either the implementation uses `is_xml_char` and naturally covers them (making the row inaccurate) or the plan contradicts its own risk mitigation by using a narrower check. This contradiction must be resolved.

**Blocking Issues:**
- Line 49: "Out of scope; issue focuses on C0 controls. Can be added later." — explicit deferral language violates ETHOS §T and the review mandate to reject any plan containing deferral language.

**Recommendations:**
- Clarify the relationship between Task 2's use of `is_xml_char` and the full scope of XML `Char` exclusions. Make maximum utility explicit.
- Specify exact test file location rather than giving two alternatives.
