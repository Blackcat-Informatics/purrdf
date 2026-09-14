# Completion-Adversary Review — Issue #294, Stage 1, Pass 1

**Verdict: FAIL (FINDINGS-OPEN)** — two substantive findings (one blocking) + one cosmetic.

---

## What I examined and compared against

- **The actual issue text** (`gh issue view 294`), not the plan's summary. The five "what closing looks like" bullets + the "over-refusal watch" clause are the contract:
  1. One `escape_xml`, not two, in `crates/rdf-core/src/iri_escape.rs`.
  2. Escape `#xD` as `&#xD;`.
  3. Refuse the **C0 scalars** XML `Char` excludes, naming the scalar.
  4. Verification = writer-then-reader, not a unit test.
  5. Moved frozen vectors / conformance rows get explained.
  Plus: `#x9`, `#xA`, `#xD`, U+0085, U+00A0, astral planes must keep round-tripping.
- **The plan** at `.stage/xml-writers-emit-literals-they-cannot/plan.md` (178 lines).
- **The prior-art assessment** (understood; the two byte-identical escapers at `rdfxml.rs`/`trix.rs` are the only in-scope duplicates).
- **The actual code**, via grep, to verify every referenced artifact:
  - `crates/rdf/src/native_codecs/rdfxml.rs` — exists; `escape_xml(value, quote)` at line 1190 (plus `escape_xml_text`/`escape_xml_attr` wrappers). Confirmed missing `#xD` and C0 handling.
  - `crates/rdf/src/native_codecs/trix.rs` — exists; `escape_xml` at line 550. Confirmed byte-identical.
  - `crates/rdf-core/src/iri_escape.rs` — exists.
  - `crates/rdf-core/src/diagnostics.rs` — **does not exist**; actual file is `diagnostic.rs` (singular).
  - `purrdf_iri::terminals::is_xml_char` — **does not exist** (see F1).
  - `purrdf::serialize` / `purrdf::parse` — **do not exist** (see F2).
  - `crates/rdf/tests/` — exists with an established round-trip-test convention.

---

## Findings (stage-1 grounds only)

### [F1] UNMAPPED-REQUIREMENT / nonexistent-artifact — the XML `Char` predicate does not exist and no task creates it

Offending plan lines:
- line 80: "Implement C0 control refusal using `purrdf_iri::terminals::is_xml_char` (the single spelled-once XML `Char` production)."
- line 174 (Risk Mitigation): "Wrong `Char` predicate | Use `purrdf_iri::terminals::is_xml_char`, not heuristic"

Evidence:
- `grep -rn "is_xml_char" crates/` → **zero matches**.
- `purrdf_iri::terminals` (`crates/iri/src/terminals.rs`) exports `is_xml_name_start_char` and `is_xml_name_char` — the XML **name** productions (`NameStartChar`/`NameChar`) — but **no `Char` (character-data) production at all**.
- The only `Char`-adjacent predicate in the workspace is the *private* `is_xml_text_char` in `crates/rdf-core/src/blank_label.rs:543`, which is `!c.is_whitespace() && matches!(c as u32, 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)`. That is **not** the `Char` production: it deliberately excludes whitespace, so it returns `false` for `#x9`/`#xA`/`#xD`, which the `Char` production *includes*.

Why it matters (the failure it leads to):
- AGENTS.md §2 terminal ring-fence is load-bearing here: "Every W3C terminal is spelled once, in `purrdf_iri::terminals`, with its production cited and its ranges asserted at compile time." `scripts/check-terminal-predicates.py` **refuses a Unicode-property/range test inside a file that holds a character cursor**. The escape code in `iri_escape.rs` is exactly such a file. So the plan's task T2, as written, has two bad resolutions: (a) hand-roll the `Char` range check in `iri_escape.rs` → the terminal-hygiene gate fails → `make check` fails → **R8 cannot be met**; or (b) realize mid-implementation that a terminal must be added to `purrdf_iri::terminals`, which the plan never budgets, cites, or compiles-time-asserts. Either way, R3 ("Refuse C0 scalars outside XML `Char`") cannot be executed against the named primitive, because the primitive is absent and no task creates it.
- Secondary contradiction folded in: T2 says "check if it's in XML `Char`; if not, return error" — which is the *full* `Char` production and would refuse `U+FFFE`/`U+FFFF` too — while Enhancements line 49 declares `U+FFFE`/`U+FFFF` "DECLINED … Can be added later". The plan never states whether the refusal predicate is "C0-only" or "all-non-`Char`", and the two mechanisms the plan names imply different answers.

REWRITE TO — insert a new task (call it T2a) before T2's escape work:
> **Task 2a — spell the `Char` terminal once.** Add to `purrdf_iri::terminals`:
> `pub const fn is_xml_char(char)` = `#x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] | [#x10000-#x10FFFF]`, with the production cited (XML 1.0 5e §2.2 `[2] Char`) and its ranges asserted at compile time (mirroring `is_xml_name_char`). Then T2's refusal is: `if !purrdf_iri::terminals::is_xml_char(scalar) → return InvalidXmlChar naming the scalar`.
> And reconcile the Enhancements table: either drop the `U+FFFE`/`U+FFFF` decline (since the full `Char` check refuses them anyway — which is more correct, not less), or state explicitly that the refusal is C0-scoped and name the distinct C0 predicate it uses.

### [F2] UNFALSIFIABLE-CRITERION — acceptance criterion names API symbols that do not exist

Offending plan line:
- line 107: "Tests use `purrdf::serialize` and `purrdf::parse`, not internal escape function"

Evidence:
- `grep -rn "pub fn serialize\b\|pub fn parse\b" crates/purrdf/src/` → only `reasoning.rs:103 parse(regime, program)`, unrelated.
- The real production surface is: `purrdf::parse_dataset(bytes, format, base)` (re-exported from `purrdf_rdf::native_codecs::parse::parse_dataset`, `crates/rdf/src/native_codecs/parse.rs:137`) and `purrdf::serialize_dataset_to_format(&dataset, format, base)` (`crates/rdf/src/native_codecs/serialize.rs:480`). These are the writer/reader the issue's own `purrdf convert` CLI wraps.

Why it matters: As literally written the check is unsatisfiable (the symbols don't exist), so an implementer could "interpret" it loosely and hand-drive `escape_xml` directly — which is exactly the unit-test-vs-production-path failure the issue's requirement 4 forbids. The *intent* (writer→reader, assert graph isomorphism) is correctly captured in R4/T3, but the named symbols are wrong.

REWRITE TO:
> "Tests call `purrdf::parse_dataset(...)` (the production reader) and `purrdf::serialize_dataset_to_format(...)` with `NativeRdfFormat::RdfXml` and `NativeRdfFormat::Trix` (the production writers), never the internal escape function."

### [F3] (minor) — nonexistent path `crates/rdf-core/src/diagnostics.rs`

Offending plan line:
- line 84: "- `crates/rdf-core/src/diagnostics.rs` (or appropriate location) — add `InvalidXmlChar` error variant"

Evidence: the file is `crates/rdf-core/src/diagnostic.rs` (singular). The plan hedges "(or appropriate location)", so this is cosmetic, not blocking.

REWRITE TO: "- `crates/rdf-core/src/diagnostic.rs` — add `InvalidXmlChar` error variant".

---

## What I deliberately did NOT flag (and why)

- **"Refuse U+FFFE/U+FFFF — DECLINED" is NOT a pre-emptive descope.** The issue's requirement 3 is "Refuse the **C0 scalars** XML's `Char` production excludes." `U+FFFE`/`U+FFFF` are not C0 scalars; the issue's own text does not list them. (The *contradiction* with T2's full-`Char` mechanism is flagged in F1, not as descope.)
- **"Add XML 1.1 support — DECLINED"** and **"Context-agnostic escape function — DECLINED"** — neither appears anywhere in the issue; declining them is legitimate boundary-setting, not laundering.
- **"Create new `xml_escape.rs` module — DECLINED"** — this *agrees* with the issue ("the workspace already has a shared home … in `crates/rdf-core/src/iri_escape.rs`"), so it's the opposite of descope.
- **The refusal in R3/T2 is NOT PLANNED-BY-REFUSAL.** This is the one issue whose headline *is* a refusal: "the serializer has to refuse"; "Refuse the C0 scalars … rather than emitting a document that cannot be read." Hard-erroring on U+0000 is the mandated deliverable, not a dodge.
- **R6 (attribute context) is an addition, not a descope** — it adds a correct, unrequested-but-in-scope check (attributes need `#x9`/`#xA`/`#xD` as refs per §3.3.3). No finding.
- **The `escape_xml_text`/`escape_xml_attr` (two functions) vs "Take context parameter (`Text` vs `Attribute`)" (one function) wording in T1** — trivial API-shape inconsistency, resolved by any competent implementation; not actionable as a finding.
- **The "or adjacent module if architecture demands" hedge in T1** — resolved by the Enhancements table committing to `iri_escape.rs`.
- **Other `escape_xml` functions in the repo (`viz/svg.rs`, `projections/util.rs`)** — out of scope: the issue is explicitly about the *two byte-identical* ones in the two XML writers (`rdfxml.rs:1190`, `trix.rs:550`). The plan scopes correctly.

---

## Confidence

High on F1 and F2 (both verified by direct grep against the workspace — no `is_xml_char`, no `purrdf::serialize`/`parse`, only `diagnostic.rs`). F1 is the blocker: the plan assumes a terminal predicate that does not exist and never schedules its creation, which under the terminal ring-fence is a hard completeness gap, not a style note. F2 and F3 are mechanical fixes to acceptance-criterion names/paths.

The plan is otherwise well-formed: every issue requirement has a contract row and a task, the verification mode (writer→reader, graph isomorphism) is the right one and matches both the issue and the `84abf779` precedent, and the goldens/conformance-explanation requirement (R7/T4) is intact.
