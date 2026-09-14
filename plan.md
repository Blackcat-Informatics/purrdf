# Implementation Plan: Issue #294 — XML Writers Emit Literals They Cannot Read Back

## Standing Constraints (from .goals)

- **GREENFIELD-FIRST**: SOTA, no compromises, learn from but do not accept previous limitations
- **RUST-FIRST**: All core functionality in high-quality Rust
- **SUBSUME, EXTEND, ENHANCE**: If something is substandard, subsume and extend it
- **LOW/NO OPTIONALITY, HARD FAILS**: Fail fast and early, no conditional imports, enforce maximum utility
- **RDF1.2 ONLY**: Never RDF 1.1, quads, or shortcuts
- **MAXIMAL UTILITY**: Err on the side of maximal utility
- **MAXIMAL PERFORMANCE**: Optimal Rust solutions, use advanced features
- **MAXIMAL PORTABILITY**: Wasm-able code as core value

## Prior Art

**First recorded instance** of XML literal escaping defects. No `GhpRsq-Defect-Class` trailers exist yet for this repo.

**Related prior work:**
- Commit `84abf779` (Sep 11 2026): Fixed predicate QName generation (same defect class, different surface). Established round-trip verification pattern (write→read→assert graph identity) but only exercised on predicates, not literals.
- Commit `7a4d9a7b` (Sep 11 2026): Collapsed five IRIREF escape transcriptions into `crates/rdf-core/src/iri_escape.rs`. Left XML literal escapers untouched because "only the PREDICATE is shared."

**Why previous fixes did not cover this:** Those fixes were scoped to predicate QNames and IRIREF bodies, respectively. XML literal character data has different rules (must escape `#xD` to survive §2.11 normalization; must refuse C0 controls excluded by XML `Char`).

**Defect-class recurrence:** This is the **third surface** of the same defect class — a duplicated escape transcription that diverges from its grammar (predicates → IRIREF → XML literals). Per the prior-art discipline, the fix is scoped to the *layer* (one spelled-once `Char` terminal + one XML egress boundary that every XML-producing surface delegates to), not merely to the two named call sites.

## Worktree State (read first)

The worktree already contains **uncommitted work-in-progress** on this exact issue (branch `paudley/294-xml-writers-emit-literals-they-cannot`, at `main` HEAD `ed03dcd1`; `.deficiencies` has no entries):

- `crates/iri/src/terminals.rs` (modified): adds the `is_xml_char` terminal via the `terminal!` macro (XML 1.0 5e §2.2 `[2] Char` cited; ranges asserted)
- `crates/rdf-core/src/xml_escape.rs` (new): `Context::{Text, Attribute}`, `InvalidXmlChar { byte_offset, character }`, `escape()` (borrowing clean path), `push()` (atomic rollback)
- `crates/rdf/src/native_codecs/{rdfxml,trix}.rs` (modified): local escapers replaced by delegation to `purrdf_core::xml_escape`
- `crates/rdf/src/projections/util.rs`, `crates/rdf/src/viz/svg.rs`, `crates/sparql-results/src/xml.rs` (modified): migrated to the same boundary
- `crates/rdf-core/src/blank_label.rs` (modified): hand-rolled `Char`-adjacent range table replaced by the terminal
- `crates/rdf/tests/xml_character_roundtrip.rs`, `crates/sparql-results/tests/xml_characters.rs` (new tests)

Each task below states what the WIP already covers and what remains. **Adopt, verify, and complete the WIP; do not rewrite it from scratch, and do not conflict with it.**

## Issue Summary

Two XML-shaped writers (`rdfxml.rs`, `trix.rs`) each carry a byte-identical `escape_xml` that is wrong in two silent ways:

1. **CR round-trips as LF**: `#xD` written raw → XML 1.0 §2.11 normalizes to `#xA` on read
2. **C0 controls produce non-well-formed XML**: Scalars outside XML `Char` (e.g., U+0000) emitted raw → exit 0 with unreadable output

## Completeness Contract

| Req # | Requirement | Task | Acceptance Check |
|-------|-------------|------|------------------|
| R1 | One `escape_xml`, not two | T1 | Both `rdfxml.rs` and `trix.rs` delegate to the shared implementation; no duplicate escape logic remains |
| R2 | Escape `#xD` as `&#xD;` | T1 | CR survives round-trip (ntriples→rdfxml→ntriples yields identical literal) |
| R3 | Refuse C0 scalars outside XML `Char` | T1a, T2 | Writer returns error naming scalar; non-zero exit; no partial output. The refusal boundary is the **full** XML 1.0 5e `Char` production (a superset of the issue's C0 scope — it also refuses U+FFFE/U+FFFF for free) |
| R4 | Round-trip verification | T3 | Production writer → production reader → assert graph identity, via `serialize_dataset_to_format` and `parse_dataset`/`dataset_from_bytes` |
| R5 | Valid neighbours keep working | T3 | `#x9`, `#xA`, `#xD`, U+007F, U+0085, U+00A0, U+D7FF/U+E000 boundary, U+FFFD, astral planes all round-trip |
| R6 | Attribute context covered | T1, T3 | `#x9`/`#xA`/`#xD` in attributes emitted as refs, survive round-trip |
| R7 | Explain moved goldens | T4 | Every non-frozen golden/conformance row with changed bytes listed in PR with reason; frozen GTS vectors untouched |
| R8 | Pass all gates | T5 | `make check` and `make wasm` pass |
| R9 | One `Char` terminal, one XML egress boundary | T1a, T1b | `is_xml_char` spelled once in `purrdf_iri::terminals`; every XML-producing surface (codecs, projections, SVG, SRX) delegates to `purrdf_core::xml_escape` |

## Enhancements Considered and Declined

| Enhancement | Decision | Reason |
|-------------|----------|--------|
| Create new `xml_escape.rs` module | **ACCEPTED** (reversal of the draft's decline) | Adjudicated contradiction: the issue text names `crates/rdf-core/src/iri_escape.rs` as the shared home, and plan-adversary (07) accepted that placement as issue-compliant; goals-reviewer-2 (04) flagged it as a blocking naming compromise and both enhancement auditors (05, 06) called IRI placement a category error — `iri_escape.rs` transcribes the RDF `IRIREF` writer terminal, while XML character-data egress is governed by different productions (XML §2.2 `Char`, §2.11 EOL, §3.3.3 attribute normalization) and a different error law. Followed 04/05/06: the issue's requirement is *one shared escape in a shared rdf-core home* (the filename was written when `iri_escape.rs` was the only egress home), and the worktree WIP already implements `purrdf_core::xml_escape` — moving it into `iri_escape.rs` would be rework that makes the code worse. |
| Refuse U+FFFE/U+FFFF (also excluded by `Char`) | **INCORPORATED — not an enhancement** | The refusal boundary is the full XML 1.0 5e `Char` production, so U+FFFE/U+FFFF are refused inherently by the same check that refuses C0 controls; declining them would require a *narrower* predicate, contradicting Task 2. Explicit test vectors for both are in Task 3's refusal matrix. (Draft's "Out of scope; Can be added later" wording removed — banned deferral language per ETHOS §T.) |
| Context-agnostic escape function | **DECLINED** | Must be context-aware (`Text` vs `Attribute`) per XML §3.3.3; a single context-free function would silently corrupt attribute whitespace |
| Add XML 1.1 support (or 1.0/1.1 mode selection) | **DECLINED** | Project uses XML 1.0 5e; 1.1 is a dead spec, and a selectable version would create two incompatible fidelity domains, violating low/no-optionality |
| Criterion benchmark for the escape path | **DECLINED** | Correctness fix with no performance claim; the escape remains a single linear scan, byte-determinism is gated by goldens, and no regression assertion is being made (raised as optional by goals-reviewer-2) |

## Implementation Tasks

### Task 1a: Spell the XML `Char` Terminal in `purrdf_iri::terminals`

**Description:**
The committed tree has **no** XML `Char` production — `purrdf_iri::terminals` exports only the *name* productions (`is_xml_name_start_char`, `is_xml_name_char`). Per the terminal ring-fence (AGENTS.md §2), the production must be spelled once, in the terminals module, before any scanner/escaper may use it. Add:

```rust
pub const fn is_xml_char(char)  // XML 1.0 Fifth Edition §2.2 production [2] Char
```

= `#x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] | [#x10000-#x10FFFF]`, via the existing `terminal!` macro, with the production cited (<https://www.w3.org/TR/xml/#NT-Char>) and its ranges asserted at compile time, mirroring the `is_xml_name_char` pattern. **Do NOT substitute `is_xml_name_char`** — `NameChar` is a strict subset of `Char` and would refuse `#x9`/`#xA`/`#xD`/`#x20`/U+0085/U+00A0, i.e. exactly the valid neighbours R5 protects.

**WIP status:** The uncommitted `crates/iri/src/terminals.rs` diff already adds exactly this block (tables `XML_CHAR_ASCII`/`XML_CHAR_NON_ASCII`). Adopt and verify it; also adopt the `blank_label.rs` hunk that replaces its hand-rolled range table with this terminal.

**Files to modify:**
- `crates/iri/src/terminals.rs` — add the `is_xml_char` terminal
- `crates/rdf-core/src/blank_label.rs` — re-express `is_xml_text_char` over the terminal (removes the second hand-rolled table)

**Acceptance:**
- `purrdf_iri::terminals::is_xml_char` exists with the production cited and ranges asserted at compile time
- `make terminal-hygiene` passes (the new terminal is registered; no hand-rolled `Char` range table remains in any cursor-holding file)
- `is_xml_char('\t')`, `is_xml_char('\n')`, `is_xml_char('\r')`, `is_xml_char('\u{7F}')`, `is_xml_char('\u{85}')`, `is_xml_char('\u{10FFFF}')` are `true`; `is_xml_char('\0')`, `is_xml_char('\u{FFFE}')`, `is_xml_char('\u{FFFF}')` are `false`

---

### Task 1: Shared XML Literal Escape in `purrdf_core::xml_escape`

**Description:**
Consolidate XML literal escaping into the dedicated module `crates/rdf-core/src/xml_escape.rs` (see Enhancements table for the adjudicated placement). The shared API must:
- Escape `&`, `<`, `>`, `"` (existing behavior)
- Escape `#xD` as `&#xD;` in character data (new — survives §2.11 normalization)
- Be context-aware (`Context::Text` vs `Context::Attribute`) — attributes additionally emit `#x9`/`#xA`/`#xD` as `&#x9;`/`&#xA;`/`&#xD;` per §3.3.3
- Validate every scalar against `purrdf_iri::terminals::is_xml_char` and return `Err(InvalidXmlChar)` naming the scalar and its byte offset
- Offer a borrowing clean path (`escape` → `Cow::Borrowed` when nothing changes) and an appending form (`push`) that leaves the caller's buffer byte-for-byte unchanged on error

**WIP status:** `xml_escape.rs` already exists with exactly this shape (`Context`, `InvalidXmlChar`, `escape`, `push`, unit tests) and `rdfxml.rs`/`trix.rs` already delegate through thin wrappers mapping `InvalidXmlChar` to `RdfDiagnostic`. Verify, complete, and keep — do not rewrite.

**Files to modify:**
- `crates/rdf-core/src/xml_escape.rs` — the shared implementation
- `crates/rdf-core/src/lib.rs` — `pub mod xml_escape;`
- `crates/rdf/src/native_codecs/rdfxml.rs` — replace local `escape_xml` with delegation
- `crates/rdf/src/native_codecs/trix.rs` — replace local `escape_xml` with delegation

**Acceptance:**
- No duplicate escape logic remains in `rdfxml.rs` or `trix.rs`
- `cargo build --workspace` succeeds
- SPDX headers on any new file

---

### Task 1b: Migrate Remaining XML Egress Surfaces to the Shared Boundary

**Description:**
The defect class is not unique to the two codecs — any hand-written XML emitter can emit a raw CR or an excluded scalar. Audit every XML-producing surface and delegate its character-data/attribute escaping to `purrdf_core::xml_escape::{escape, push}` with the correct `Context`. Known surfaces: GraphML/DataCite/RO-Crate projection utilities (`crates/rdf/src/projections/util.rs`), SVG output (`crates/rdf/src/viz/svg.rs`), and SPARQL Results XML (`crates/sparql-results/src/xml.rs`). Each surface maps `InvalidXmlChar` into its own established error family without dropping the scalar or the byte offset.

**WIP status:** All three named surfaces are already migrated in the uncommitted diff. Verify the delegation is total (no residual local escaper) and complete any missed call site.

**Files to modify:**
- `crates/rdf/src/projections/util.rs`
- `crates/rdf/src/viz/svg.rs`
- `crates/sparql-results/src/xml.rs`

**Acceptance:**
- Repository-wide search finds no XML escaping outside `purrdf_core::xml_escape` (thin format-local wrappers that only convert the error type are acceptable)
- SRX round-trip test (`crates/sparql-results/tests/xml_characters.rs`, already in WIP) passes

---

### Task 2: Invalid Character Refusal

**Description:**
Refusal is against the **full XML 1.0 5e `Char` production** via `purrdf_iri::terminals::is_xml_char` (Task 1a) — before writing any scalar, check membership in `Char`; if excluded, return an error naming the scalar. This inherently covers the issue's C0 controls **and** U+FFFE/U+FFFF (surrogates cannot inhabit a Rust `char`).

**Atomic semantics (explicit, not "if feasible"):** validate-then-write. The appending form (`push`) truncates the caller's buffer back to its original length on error; both codecs construct the complete document and only then append it in `RdfCodec::serialize_into`, so the public serialization boundary never yields a partial artifact.

**Error provenance:** `InvalidXmlChar { character, byte_offset }` lives in `purrdf_core::xml_escape`; the byte offset is **required** (it is cheap — `char_indices` provides it). Format layers convert it into their established serialization error (`RdfDiagnostic` for the codecs) preserving both fields.

**Files to modify:**
- `crates/rdf-core/src/xml_escape.rs` — validation inside `escape`/`push`
- `crates/rdf-core/src/diagnostic.rs` (singular — not `diagnostics.rs`) — any needed diagnostic plumbing

**Acceptance:**
- U+0000 in a literal produces an error naming the scalar: e.g. "U+0000 at byte N is not permitted in XML 1.0"
- U+FFFE and U+FFFF produce the same class of error (covered by the same `Char` check — no special-casing)
- Error carries the byte offset; first invalid scalar after a multibyte valid prefix is reported correctly
- No output written before error: `push` rollback is byte-exact, and `serialize_into` with a nonempty caller buffer leaves it unchanged on failure

---

### Task 3: Round-Trip Integration Tests

**Description:**
Integration tests use the **production** writer and reader — `purrdf::serialize_dataset_to_format` / `purrdf::parse_dataset` (or `purrdf::dataset_from_bytes`) with `NativeRdfFormat::RdfXml` and `NativeRdfFormat::TriX` — never the internal escape function directly. (`purrdf::serialize` and `purrdf::parse` do not exist.) Coverage:

- **CR round-trip:** `"a\r\nb"` survives unchanged; emitted document contains no raw CR
- **Attribute whitespace:** tab/LF/CR emitted as `&#x9;`/`&#xA;`/`&#xD;` and round-trip, exercised on a production attribute-bearing path (the RDF/XML `rdf:parseType="Literal"` canonicalization path, whose embedded element attributes carry arbitrary whitespace — `xml:lang`/datatype IRIs cannot)
- **Valid neighbours:** `#x9`, `#xA`, `#xD`, U+007F, U+0085, U+00A0, U+D7FF/U+E000 boundary, U+FFFD, astral scalars (U+10000, U+1FFFE, U+10FFFF)
- **Refusal matrix:** every C0 scalar except `#x9`/`#xA`/`#xD` (i.e. U+0000–U+0008, U+000B, U+000C, U+000E–U+001F), plus U+FFFE and U+FFFF, each producing a scalar-naming error
- **Well-formedness oracle:** emitted bytes parse with the independent XML reader (`roxmltree`) before the semantic re-read
- **Comparison:** re-read dataset re-serialized to N-Triples compares byte-equal to the original's N-Triples (graph isomorphism where blank-node identity can differ)

**WIP status:** `crates/rdf/tests/xml_character_roundtrip.rs` already implements the valid-scalar matrix, the full refusal matrix (all C0 + U+FFFE/U+FFFF), and the XMLLiteral attribute-whitespace case, exactly through the production surfaces named above. Adopt and extend only if a listed case is missing. SRX coverage lives in `crates/sparql-results/tests/xml_characters.rs`.

**Files to create/modify:**
- `crates/rdf/tests/xml_character_roundtrip.rs` — the RDF/XML + TriX suite (committed location; no alternative)
- `crates/sparql-results/tests/xml_characters.rs` — the SRX suite

**Acceptance:**
- Tests call `serialize_dataset_to_format` (writer) and `parse_dataset`/`dataset_from_bytes` (reader) — never the internal escape function
- Tests assert dataset identity (N-Triples byte equality / isomorphism), not string equality of XML
- RDF/XML, TriX, and SRX surfaces all tested; U+FFFE/U+FFFF refusal proven
- Append-level atomicity: a failing `push`/serialize leaves pre-existing buffer content unchanged

---

### Task 4: Golden/Conformance Audit

**Description:**
Run the full test suite; identify any goldens or conformance rows whose bytes change. Distinguish the artifact classes precisely:

- **Frozen GTS vectors** (`vectors/`): must **never** be regenerated or "fixed" here — the GTS wire format is governed in `gmeow-gts`. If any would change, stop; that is a defect, not a refresh.
- **Generated projections** (`generated/`): regenerate only via `make metadata`, never hand-edit.
- **Ordinary goldens / conformance rows** (RDF/XML, TriX fixtures): enumerate every changed file with before/after hash and the reason ("CR in literal now escaped as `&#xD;`" / "C0 scalar now refused").

**Acceptance:**
- List of every moved golden/row with before/after hash and per-file explanation
- No silent regeneration of vectors; frozen GTS vectors untouched
- If no RDF/XML/TriX golden actually contains a raw CR or excluded scalar, say so explicitly rather than manufacturing churn

---

### Task 5: Final Verification

**Description:**
Run the repository's canonical gates — not a hand-assembled subset.

**Commands:**
```bash
make check   # fmt, clippy (pedantic + nursery), build, tests, hygiene (incl. terminal-hygiene, build-profile-hygiene, check-no-features, check-generated)
make wasm    # wasm32-unknown-unknown for every release crate
```

**Acceptance:**
- Both commands exit 0
- No clippy warnings

---

### Task 6: PR Creation

**Description:**
Create PR linking to issue #294.

**PR body includes:**
- Summary of changes
- List of moved goldens with reasons
- Verification: round-trip tests pass
- Compliance: `make check` and `make wasm` pass

## Success Criteria

- [ ] `is_xml_char` spelled once in `purrdf_iri::terminals` with production cited; `make terminal-hygiene` passes
- [ ] One shared escape implementation in `purrdf_core::xml_escape` (no duplication in any XML-producing surface)
- [ ] CR round-trips correctly through RDF/XML
- [ ] CR round-trips correctly through TriX
- [ ] U+0000 produces hard error naming the scalar and byte offset
- [ ] U+FFFE and U+FFFF produce hard errors (same `Char` check)
- [ ] `#x9`, `#xA`, `#xD` in attributes round-trip (including the XMLLiteral embedded-attribute path)
- [ ] Valid neighbours (`#x9`, `#xA`, `#xD`, U+007F, U+0085, U+00A0, U+D7FF/U+E000, U+FFFD, astral) all pass
- [ ] Integration tests use the production writer→reader surfaces (`serialize_dataset_to_format` / `parse_dataset` / `dataset_from_bytes`)
- [ ] All moved goldens explained in PR; frozen GTS vectors untouched
- [ ] `make check` and `make wasm` pass
- [ ] SPDX headers present

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Over-refusal of valid chars | Full `Char` production (not `NameChar` — a strict subset that would refuse `#x9`/`#xA`/`#xD`/space); boundary scalars tested explicitly in T3 |
| Under-refusal (U+FFFE/U+FFFF) | Same `Char` check refuses them; refusal matrix in T3 proves it |
| Partial output on error | Validate-then-write; `push` truncates to the caller's original length; codecs build the whole document before appending |
| Wrong `Char` predicate | Task 1a spells it once in `purrdf_iri::terminals` with compile-time-asserted ranges; terminal-hygiene gate refuses hand-rolled tables |
| Text/Attribute confusion | Context-aware API with `Context` enum; §3.3.3 rules only in `Attribute` |

## Assembler Note

The eight reviews split on a factual axis: the enhancement auditors (05, 06) examined this worktree's **uncommitted** WIP (which already contains `is_xml_char`, `xml_escape.rs`, and the codec/projection/SVG/SRX migrations), while both plan-adversaries (07, 08) verified against the **committed** tree (where none of that exists) — hence F1's "predicate does not exist." Both are right about their respective trees; the plan is written against the committed baseline and explicitly directs the implementer to adopt and verify the WIP. The module placement (`xml_escape.rs`, not the issue-named `iri_escape.rs`) is the one deviation from literal issue text — adjudicated as technical (see Enhancements table); the operator may veto.

---
*Plan created for issue #294 — XML writers emit literals they cannot read back*
