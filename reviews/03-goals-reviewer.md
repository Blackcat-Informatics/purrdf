# Goals Review: Issue #294 Plan

**Plan:** `/home/paudley/Active/purrdf/.worktrees/294-xml-writers-emit-literals-they-cannot/.stage/xml-writers-emit-literals-they-cannot/plan.md`
**Reviewed:** 2026-09-14
**Reviewer:** Goals Reviewer (Alibaba Model)

---

## Goals Compliance Assessment

### GREENFIELD-FIRST — PASS

The plan demonstrates SOTA thinking:
- Consolidates two duplicate `escape_xml` implementations into one shared module (R1), learning from the fragmentation that allowed this defect to persist
- Extends existing `iri_escape.rs` rather than creating Yet Another Module, showing mature architectural judgment
- Addresses the root cause (XML §2.11 normalization + C0 controls) rather than papering over symptoms
- Acknowledges prior art (`84abf779`, `7a4d9a7b`) and explains why they didn't cover this case

The plan correctly identifies this as the first XML literal escaping defect and establishes the round-trip verification pattern as the standard.

---

### RUST-FIRST — PASS

All core functionality is Rust:
- Implementation targets `crates/rdf-core/src/iri_escape.rs` (Rust)
- Writers in `crates/rdf/src/native_codecs/` (Rust)
- Integration tests in `crates/rdf/tests/` (Rust)
- No Python surface mentioned — this is pure library infrastructure
- Uses idiomatic Rust patterns (`Result<(), XmlEscapeError>`, buffer-based escaping)

---

### SUBSUME, EXTEND, ENHANCE — PASS

Excellent adherence:
- **Subsume**: Consolidates `escape_xml` from `rdfxml.rs` and `trix.rs` into shared implementation
- **Extend**: Adds XML literal escaping to existing `iri_escape.rs` module
- **Enhance**: The "Enhancements Considered and Declined" table shows the plan specifically declined to create a new `xml_escape.rs` module, instead extending the existing escape infrastructure — exactly the right pattern

The "Declined" section is particularly well-reasoned:
- Declined new module (extend instead)
- Declined XML 1.1 support (dead spec)
- Declined context-agnostic escape (XML spec requires context awareness)

---

### LOW/NO OPTIONALITY, HARD FAILS — PASS

Exceptional alignment:
- **Hard failure for invalid C0 controls**: R3 requires "Writer returns error naming scalar; non-zero exit; no partial output"
- **No silent degradation**: Current bug is exactly the kind of silent failure this goal prohibits — plan fixes it properly
- **No optional features**: The "Enhancements Considered and Declined" table explicitly rejects optionality (U+FFFE/U+FFFF refusal "can be added later", XML 1.1 support)
- **Result type**: Uses `Result<(), XmlEscapeError>` not `Option` or silent skip
- **Atomic semantics**: Task 2 acceptance requires "No output written before error"

The current bug (CR→LF, C0 controls producing invalid XML) is the archetype of what this goal forbids. The fix is the archetype of what it requires.

---

### RDF1.2 ONLY — PASS

Plan focuses on XML serialization (RDF/XML, TriX) which are valid RDF 1.2 syntaxes:
- Mentions N-Triples only for round-trip testing (a test tool, not a data model)
- Graph isomorphism verification (RDF 1.2 semantics)
- No quads, no RDF 1.1 shortcuts
- "astral planes" test coverage ensures full Unicode scalar support (RDF 1.2 requirement)

---

### MAXIMAL UTILITY — PASS

Plan errs on the side of comprehensive:
- Both Text and Attribute contexts handled (R6)
- Full range of valid characters tested: `#x9`, `#xA`, `#xD`, U+0085, U+00A0, astral planes (R5)
- Invalid scalars explicitly refused, not silently corrupted
- Round-trip verification through production writer→reader (R4)
- Golden/conformance audit ensures no regressions (T4)

The "Enhancements Declined" table shows restraint — utility is maximal within scope, not scope-creep.

---

### MAXIMAL PERFORMANCE — PASS

Performance-conscious decisions:
- Single shared implementation (eliminates duplication)
- Uses `purrdf_iri::terminals::is_xml_char` (spelled-once predicate, no recomputation)
- Validate-then-write pattern (avoids partial output + rollback costs)
- Buffer-based escaping (avoids per-character allocation)
- Context-aware API (avoids re-scanning for attribute vs text detection)

No performance pessimizations identified.

---

### MAXIMAL PORTABILITY — PASS

- Pure Rust text processing — no platform dependencies
- Full Unicode support (astral planes in test coverage)
- `make wasm` in final verification (T5)
- No file I/O in escape logic (buffer-based)

---

## Risk Assessment

| Risk | Assessment |
|------|------------|
| Over-refusal | Mitigated by explicit test coverage of valid characters (R5) |
| Partial output | Mitigated by validate-then-write / atomic semantics requirement |
| Wrong predicate | Mitigated by using `purrdf_iri::terminals::is_xml_char` |
| Text/Attribute confusion | Mitigated by context-aware API with enum parameter |

---

## Summary

This plan demonstrates **exceptional** alignment with all stated goals. The "Enhancements Considered and Declined" section is a model of how to respect "LOW/NO OPTIONALITY" — it documents what was considered and explicitly rejected with reasoned justification.

The fix for "writers emit literals they cannot read back" is exactly the kind of hard-fail, no-compromise approach the goals mandate. The consolidation of duplicate code into a shared, extended implementation follows SUBSUME, EXTEND, ENHANCE perfectly.

---

**Verdict: PASS**

**Overall Assessment:** The plan is well-architected, goal-compliant, and ready for implementation. The explicit rejection of optionality in the "Enhancements Declined" section is particularly commendable.
