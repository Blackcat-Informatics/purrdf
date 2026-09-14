# Plan Adversary (pass 2, xAI Grok) — Issue #294, Stage 1

**Verdict: FAIL** — three findings reference artifacts that do not exist in the
workspace, and one internal contradiction. Two are blockers: the plan cannot be
executed as written because it names a terminal predicate that does not exist and
names public API functions that do not exist.

---

## What I examined

- The plan: `.stage/.../plan.md` (full, 178 lines).
- The prior-art assessment: `.stage/.../prior-art-assessment.md` (full, 65 lines).
- The completion-adversary doctrine (`~/.config/opencode/commands/includes/_completion-adversary.md`).
- Issue #294 ACTUAL text via `gh issue view 294 --json title,body,comments` (body only; no comments).
- Verified referenced artifacts against the live repo (main worktree at
  `/home/paudley/Active/purrdf` — the source files under `crates/` are shared
  between the main checkout and the worktree; both read the same tree here).

Cross-checks performed (all greps run, results quoted where material):

| Plan claim | Reality | Result |
|---|---|---|
| `purrdf_iri::terminals::is_xml_char` ("the single spelled-once XML `Char` production") | `grep -rn "is_xml_char" crates/` → exit 1 (no match). terminals.rs has `is_xml_name_start_char` / `is_xml_name_char` only, plus IRIREF/PN predicates. No `Char` production spelled anywhere. | **MISSING** |
| `crates/rdf-core/src/diagnostics.rs` | File does not exist; the crate has `diagnostic.rs` (singular). | **MISSING** |
| `purrdf::serialize` and `purrdf::parse` | Neither exists. `pub use purrdf_rdf::*` re-exports `serialize_dataset`, `serialize_dataset_to_format`, `parse_dataset`. The only `pub fn parse` is `reasoning.rs:103` (`ReasoningProgram::parse`). | **MISSING** |
| `escape_xml` duplicated at `rdfxml.rs:1190` / `trix.rs:550` | Confirmed byte-identical cores at exactly those lines. | correct |
| `iri_escape.rs` exists and is the shared egress home | Confirmed (`crates/rdf-core/src/iri_escape.rs`, 8.3KB, `is_iriref_escape_required` only). | correct |

---

## Findings

### [F1] BLOCKER — Task 2 names a terminal predicate that does not exist, and the nearest real one would break the issue's own over-refusal watch

Plan line 80 (Task 2):

> "Implement C0 control refusal using `purrdf_iri::terminals::is_xml_char` (the single spelled-once XML `Char` production)."

`is_xml_char` does not exist anywhere in `crates/`. The XML `Char` production
(`Char ::= #x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] | [#x10000-#x10FFFF]`)
is **not** spelled in `purrdf_iri::terminals`. The only XML predicates present are
`is_xml_name_start_char` and `is_xml_name_char`, which govern **NAME characters** —
a strict subset of `Char` that excludes `#x9`, `#xA`, `#xD`, `#x20`, U+0085, U+00A0,
and every punctuation/space scalar.

Why this is a blocker, not a nit:

1. **The plan cannot execute as written.** There is no `is_xml_char` to import.
2. **The parenthetical is false.** It claims the `Char` production is already
   "spelled once" in the terminals module. It is spelled zero times. The
   terminal-ring-fence doctrine (`AGENTS.md` §2) requires every W3C terminal to be
   spelled once with its production cited and ranges asserted at compile time — so
   this is *new* terminal work the plan must schedule, including the
   `scripts/check-terminal-predicates.py` hygiene gate consequences. The plan
   schedules none of it because it believes the predicate pre-exists.
3. **The plausible wrong substitute is actively harmful.** An implementer reaching
   for the closest-sounding existing symbol (`is_xml_name_char`) would refuse
   `#x9`, `#xA`, `#xD`, `#x20`, U+0085, U+00A0 — i.e. exactly the scalars the
   issue's "Over-refusal watch" paragraph and the plan's own R5 demand keep
   round-tripping. The failure mode of the plan's error is a silent over-refusal of
   the very neighbours the issue calls out.

**REWRITE TO** (Task 2, new first step, plus a new task or expansion of T1):

> Implement C0 refusal against a **new** `is_xml_char` predicate authored in
> `purrdf_iri::terminals` for XML 1.0 5e §2.2 `[2]` `Char`, spelled once with its
> production cited and its ranges asserted at compile time (mirroring the
> `is_xml_name_char` pattern), and re-run `make terminal-hygiene` to register the
> new terminal. Do NOT use `is_xml_name_char` — `NameChar` is a strict subset of
> `Char` and would refuse `#x9/#xA/#xD/#x20/U+0085/U+00A0`.

(Confidence: high. `is_xml_char` absent from the whole tree; `Char` production absent from terminals.rs.)

---

### [F2] BLOCKER — Task 3's acceptance criterion names public API functions that do not exist

Plan line 107 (Task 3 acceptance):

> "Tests use `purrdf::serialize` and `purrdf::parse`, not internal escape function"

Neither `purrdf::serialize` nor `purrdf::parse` exists. The production writer/reader
entry points are `purrdf::serialize_dataset` / `purrdf::serialize_dataset_to_format`
and `purrdf::parse_dataset` (re-exported via `pub use purrdf_rdf::*` →
`native_codecs::{serialize_dataset, parse_dataset, ...}`). The only `pub fn parse`
in the facade is `purrdf::reasoning::ReasoningProgram::parse`, unrelated to RDF
codecs.

Why it matters: this is the acceptance check for R4 — the issue's headline
requirement ("Verification is writer then reader, not a unit test"). If the
implementer follows the plan literally they cannot find the named symbols, and the
fallback ("or just call the codec internals") drifts back toward a unit test
against `escape_xml_*`, which the issue explicitly rules out. The *intent* is
right; the *named surface* is wrong.

**REWRITE TO** (line 107):

> Tests call `purrdf::serialize_dataset` (format = RDF/XML, then TriX) and
> `purrdf::parse_dataset` — the production writer/reader — never the internal
> escape function, and assert graph isomorphism.

(Confidence: high. Verified via `grep -rn "pub fn serialize\b\|pub fn parse\b"` across `crates/rdf/src` and `crates/purrdf/src`.)

---

### [F3] — Task 2 names a nonexistent file path

Plan line 84 (Task 2, Files to modify):

> "`crates/rdf-core/src/diagnostics.rs` (or appropriate location) — add `InvalidXmlChar` error variant"

The file is `crates/rdf-core/src/diagnostic.rs` (singular). The `(or appropriate
location)` hedge softens but does not cure it: a plan step that names a wrong path
is a plan step against a nonexistent path.

**REWRITE TO** (line 84):

> "`crates/rdf-core/src/diagnostic.rs` — add an `InvalidXmlChar` diagnostic/error
> carrying the offending scalar (and byte offset when available)".

(Confidence: high. `ls crates/rdf-core/src/` shows `diagnostic.rs`, no `diagnostics.rs`.)

---

### [F4] — Internal contradiction on the refusal boundary (U+FFFE/U+FFFF)

Plan line 80 (Task 2) says:

> "Before writing any scalar, check if it's in XML `Char`; if not, return error naming the scalar."

Plan line 49 (Enhancements Declined) says:

> "Refuse U+FFFE/U+FFFF (also excluded by `Char`) | DECLINED | Out of scope; issue focuses on C0 controls."

These cannot both be true as written. A full `is_xml_char` (Char-production) check
refuses U+FFFE/U+FFFF *for free* — they are outside `Char` — so "declining" them is
not a code reduction, it is at most a decision not to add explicit test vectors. If
the implementation instead checks only C0 (a narrower set), then it is not doing
"check if it's in XML Char", contradicting line 80. The refusal boundary is the core
of this change and must be stated once.

This is *not* a descope of an issue-listed item (the issue scopes to "C0 scalars";
U+FFFE/U+FFFF are not C0), so I do not flag it as PRE-EMPTIVE-DESCOPE. It is a
self-contradiction that leaves the implementer free to pick either boundary, and one
of them (C0-only) silently leaves the headline "emit literals they cannot read back"
partly unfixed for the non-C0 Char exclusions.

**REWRITE TO** (reconcile the two lines):

> Task 2: "Before writing any scalar, refuse anything outside XML `Char` (this
> inherently covers C0 controls AND U+FFFE/U+FFFF)." Declined table: replace the
> U+FFFE/U+FFFF row with "Add explicit U+FFFE/U+FFFF *test vectors* — DECLINED
> (the Char-production check already refuses them; issue scopes tests to C0)."

(Confidence: medium-high on the contradiction; the recommended resolution is the
only one that makes both lines simultaneously true.)

---

## What I deliberately did NOT flag (and why)

- **The signature "Return `Result<(), XmlEscapeError>`" (Task 1)** — an escape
  function must also yield the escaped string (`Result<Cow<str>, _>` is the obvious
  shape). This is a missing-detail in a task whose acceptance criterion is already
  executable, which the stage-1 doctrine explicitly excludes. Noted for the
  implementer, not a finding.
- **The attribute-context requirement (R6, `#x9/#xA/#xD` → refs per §3.3.3)** — the
  issue does not spell this out explicitly, but it is *required* for the issue's
  "round-trip" and "over-refusal watch" to be honest (raw `#x9/#xA/#xD` in attribute
  values normalize to `#x20` on read). The plan correctly adds it. Not a finding; it
  is the plan being more complete than the issue, not less.
- **The `#xD`-as-`&#xD;` and C0-refusal behavior in text vs attribute** — already
  covered by R2/R3/R6 and consistent with the issue.
- **`make check` vs the four-command list in Task 5** — R8's parenthetical "(fmt,
  clippy, tests, wasm)" understates what `make check` includes (`terminal-hygiene`,
  `build-profile-hygiene`, `check-no-features`, `check-generated`). This only becomes
  load-bearing *because of F1* (authoring a new terminal predicate triggers the
  terminal-hygiene gate). Folded into F1's remediation rather than raised separately.
- **Naming divergence between call sites** (`escape_xml_text`/`escape_xml_attr` in
  rdfxml.rs vs `escape_text`/`escape_attr` in trix.rs) — the duplicated *core*
  (`escape_xml(value, quote)`) is what the issue means by "byte-identical"; the
  wrapper names differing is cosmetic and doesn't change scope.

---

## Completeness Contract vs issue requirements

All six issue requirements are mapped to a row, and every row points at a task that
exists (T1–T6 all present):

| Issue requirement | Row | Mapped? |
|---|---|---|
| One `escape_xml`, not two (shared in rdf-core) | R1 → T1 | yes |
| Escape `#xD` as `&#xD;` | R2 → T1 | yes |
| Refuse C0 scalars outside `Char`, naming scalar | R3 → T2 | yes (but T2's named predicate is nonexistent — F1) |
| Writer-then-reader verification, not unit test | R4 → T3 | yes (but T3's named API is nonexistent — F2) |
| Explain moved frozen vectors/conformance rows | R7 → T4 | yes |
| Valid neighbours keep round-tripping (over-refusal watch) | R5 → T3 | yes |

No UNMAPPED requirement, no PRE-EMPTIVE DESCOPE of an issue-listed item, no
PLANNED-BY-REFUSAL (the C0 refusal *is* the requirement, not a refusal of it). The
failures are artifact-reference failures (F1–F3) and one self-contradiction (F4).

---

## Net assessment

The plan's *architecture* is sound and its mapping to the issue is complete and
correct in intent. Its *executability* is broken in two places: it assumes a
terminal predicate (`is_xml_char`) exists when the `Char` production has never been
spelled in this repo, and it names `purrdf::serialize`/`purrdf::parse` which are not
the real writer/reader entry points. F1 and F2 are both fixable by rewriting the
task text (and, for F1, actually scheduling the terminal-authoring work), but as
submitted the plan would lead an implementer to either a false import or a
unit-test drift on the exact requirement the issue calls most important.
