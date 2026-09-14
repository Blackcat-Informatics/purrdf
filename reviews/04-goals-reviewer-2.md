# Goals Review – Issue #294 Plan

**Reviewer:** goals-reviewer-2 (MiniMax)  
**Plan:** `.stage/xml-writers-emit-literals-they-cannot/plan.md`  
**Date:** 2026-09-14  

---

## Executive Summary

The plan correctly identifies the bug, proposes a shared implementation (R1), hard-fails on invalid scalars (R3), and respects the RDF 1.2-only mandate. However, it contains **one banned deferral pattern** and **two organizational compromises** that place it in tension with `.goals`.

**Verdict:** **FAIL** – blocking issues must be resolved before execution.

---

## Goal-by-Goal Assessment

### 1. GREENFIELD-FIRST — LARGELY PASS

The plan learns from predicate-QName fixes (commit `84abf779`) and the IRIREF collapse (commit `7a4d9a7b`), explicitly stating why those fixes did not cover XML literal character data. It does not accept the prior limitation.

- **Why not a clean PASS:** The prior fixes left XML literal escapers untouched with the rationale "only the PREDICATE is shared." The plan adopts the same shared-module consolidation strategy but applies it to `iri_escape.rs`, a module named for IRI escaping, not XML character escaping. This is a structural compromise that the plan accepts without renaming or reorganizing, which borders on accepting prior limitations.

### 2. RUST-FIRST — PASS

All implementation is in Rust (`crates/rdf-core/src/iri_escape.rs`, `crates/rdf/src/native_codecs/rdfxml.rs`, `trix.rs`). No foreign surface is proposed for core logic. The error type (`XmlEscapeError`) and context enum are idiomatic Rust.

### 3. SUBSUME, EXTEND, ENHANCE — CONCERN

The plan **subsumes** the duplicate `escape_xml` functions. It **extends** `iri_escape.rs`. It does **not** enhance the module naming.

`iri_escape.rs` is the "single spelled-once" home for IRI escaping. XML literal escaping is semantically different and is not an IRI operation. `.goals` says: "if something we use is substandard or holding us back, subsume its functionality, extend it ruthlessly, then enhance it well beyond the original." A module named `iri_escape` that also contains XML text/attribute escaping is substandard organization. The plan explicitly **declined** creating a new `xml_escape.rs` module "per issue spec," which suggests an external constraint is overriding the internal quality bar.

**Recommendation:** Either rename `iri_escape.rs` to a generic escape home (e.g., `escape.rs`) or create `xml_escape.rs` and move the shared logic there. If the issue spec truly forbids a new file, the issue spec is at odds with `.goals`; the `.goals` file takes precedence because it governs every PR in this repo.

### 4. LOW/NO OPTIONALITY, HARD FAILS — PASS WITH FLAG

The plan correctly proposes hard failure on invalid scalars (R3: "Writer returns error naming scalar; non-zero exit; no partial output"). There are no conditional imports, no optional dependencies, and no "if available" checks.

However, the "Enhancements Considered and Declined" table introduces optionality by scope:

| Item | Decision | Problem |
|------|----------|---------|
| Refuse U+FFFE/U+FFFF | **DECLINED** | These scalars are **also** excluded by XML 1.0 `Char`. Refusing them is not an "enhancement"; it is the **correct and complete** behavior of the `is_xml_char` predicate the plan says it will use. Declining it is partial implementation of a predicate that already exists. |
| Context-agnostic escape function | **DECLINED** | Correctly declined; XML §3.3.3 demands context-awareness. This is not optionality, it is spec compliance. |
| XML 1.1 support | **DECLINED** | Correctly declined; 1.1 is dead. |

**Critical:** The plan says Task 2 will "Implement C0 control refusal using `purrdf_iri::terminals::is_xml_char`." If the implementation actually uses `is_xml_char`, it **will** refuse U+FFFE/U+FFFF automatically, because those scalars fail the predicate. The plan therefore has an internal contradiction: it claims to use a comprehensive predicate but only advertises partial coverage, and then declines the full coverage as "out of scope."

This is a documentation/test-plan bug that will confuse the implementer and the reviewer. It must be fixed: Task 2 should state that **all** scalars outside XML 1.0 `Char` are refused, and the test plan should include U+FFFE/U+FFFF.

### 5. RDF 1.2 ONLY — PASS

No RDF 1.1, quads, or shortcuts are proposed. The plan operates exclusively on RDF/XML and TriX serializations of RDF 1.2 datasets.

### 6. MAXIMAL UTILITY — CONCERN / NEAR-VIOLATION

Three issues:

1. **Partial Char enforcement.** As noted above, using `is_xml_char` but only testing/documenting C0 controls is not maximal utility. Maximal utility means: if the predicate exists and is correct, use it fully and test it fully.

2. **Deferred work language.** The table explicitly states: "Can be added later." This phrase is banned under ETHOS §T (Total Ownership): "Work is NOW: banned resolutions — 'deferred', 'follow-up', 'phase 2', 'for now'." `.goals` enforces this ethos. The plan must either (a) include U+FFFE/U+FFFF refusal now, or (b) state explicitly why those scalars are handled differently (which they are not, by the predicate).

3. **Task 5 verification gap.** The plan says "Run full gate" but lists individual commands (`cargo fmt --check`, `cargo clippy`, `cargo test`, `make wasm`). The actual full gate is `make check`, which includes `make build-profile-hygiene`, `make terminal-hygiene`, and other checks defined in `AGENTS.md`. Listing subcommands instead of the canonical gate is a minor deviation from "One Path" (ETHOS §O) and risks missing a hygiene check.

### 7. MAXIMAL PERFORMANCE — PASS

The plan avoids per-scalar allocation where possible (mentions "use buffer if needed" for atomic error semantics). It references the existing `is_xml_char` predicate rather than reimplementing. No performance degradation is introduced.

One note: The plan does not benchmark the new escape path. For a serialization hot path, a micro-benchmark comparing old vs. new `escape_xml` would be advisable, though not mandatory for a correctness fix.

### 8. MAXIMAL PORTABILITY — PASS

No new dependencies, no system calls, no threads, no filesystem access. The changes are contained to `purrdf-core` and `purrdf-rdf`, both of which are already wasm-able. No risk to `wasm32-unknown-unknown`.

---

## Blocking Issues (must fix before execution)

1. **[MAXIMAL UTILITY / ETHOS §T] Remove "Can be added later"**
   - Location: "Enhancements Considered and Declined" table, row 2.
   - Fix: Either include U+FFFE/U+FFFF refusal in the completeness contract (R3) and test plan (T3), or remove the `is_xml_char` reference and use a C0-only predicate. Using `is_xml_char` partially is worse than using a focused predicate fully.

2. **[SUBSUME, EXTEND, ENHANCE] Resolve module naming compromise**
   - Location: Task 1.
   - Fix: If the shared escape logic lives in `iri_escape.rs`, rename the module or explain why `iri_escape` is semantically the right home for XML literal escaping. Do not accept a misleading module name because of an "issue spec" constraint.

3. **[ETHOS §O] Use the canonical gate command**
   - Location: Task 5.
   - Fix: Replace the command list with `make check`, which is the defined full gate in `AGENTS.md`.

---

## Non-Blocking Recommendations

1. **Clarify atomic semantics.** Task 2 says "No output written before error (atomic semantics)" but also says "use buffer if needed." Be explicit: will the implementation buffer the entire literal before writing, or stream with early validation? Either is fine, but the plan should state the chosen strategy.

2. **Add U+FFFE/U+FFFF to R3/T3.** Even if treated as blocking above, if the plan author believes these are truly edge cases that never occur in practice, they should still be tested because `is_xml_char` will catch them. A test that asserts the error message for U+FFFE costs nothing and proves the predicate is used correctly.

3. **Performance note.** Add a criterion benchmark for `escape_xml_text` / `escape_xml_attr` on a 1KB literal with mixed content (CR, LF, `<`, `>`, astral). This establishes a baseline and prevents regressions in a hot path.

---

## Overall Assessment

The plan is **technically competent** and **correctness-focused**, but it makes two compromises that `.goals` forbids: (1) a banned deferral phrase that signals partial delivery, and (2) accepting a suboptimal module organization without enhancement. Fixing these is low-effort and high-value. Once the blocking items are addressed, the plan should proceed.

**Status:** **FAIL** — needs revision.

**Required actions before re-review:**
1. Remove or replace the "Can be added later" language for U+FFFE/U+FFFF.
2. Clarify whether `is_xml_char` is used fully (and test U+FFFE/U+FFFF) or replaced with a C0-only check.
3. Rename `iri_escape.rs` or justify why it is the correct home for XML escaping.
4. Change Task 5 commands to `make check`.
