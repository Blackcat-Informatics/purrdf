# Issue Analysis: #294

**Title:** fix(rdf): the XML writers emit literals they cannot read back
**State:** OPEN
**Repo:** Blackcat-Informatics/purrdf
**Forge:** github
**Author:** (not stated in the staged file)
**Created:** (not stated in the staged file)
**Labels:** (not stated in the staged file)
**Comments:** 0

> Source of truth for this analysis is the staged file
> `.stage/xml-writers-emit-literals-they-cannot/issue.md`. No retrieval commands
> were run; line references below were confirmed against the checked-out
> worktree only to anchor "Affected Areas".

---

## Summary

The two XML-shaped writers — RDF/XML (`crates/rdf/src/native_codecs/rdfxml.rs`)
and TriX (`crates/rdf/src/native_codecs/trix.rs`) — each carry a byte-identical
transcription of an XML escaping routine that is wrong in two silent ways. It
writes a literal carriage return raw, so XML 1.0 §2.11 parser normalization turns
`#xD` into `#xA` and a CR-bearing literal round-trips as a *different* literal;
and it writes C0 control scalars raw, producing, at exit zero, a document the
project's own reader rejects and no conforming XML parser accepts. The issue is
filed separately from the scanner-terminal work that found it because the fix
**changes emitted bytes** and introduces a **new hard error**, either of which may
legitimately move a frozen vector or conformance row. Closing it means one shared
escape, `&#xD;` for CR, an explicit refusal (naming the scalar) for scalars
outside XML's `Char` production, and writer-then-reader round-trip verification.

---

## Requirements

### Explicit Requirements

1. **One `escape_xml`, not two.** Collapse the duplicated transcription in the
   RDF/XML and TriX writers onto a single shared implementation. The issue names
   the workspace's existing shared egress home as
   `crates/rdf-core/src/iri_escape.rs`. (See "Current Repository State" — the
   working tree appears to host this in a new `crates/rdf-core/src/xml_escape.rs`
   instead; the *law* is shared, the file location is an implementation detail to
   settle in the plan.)
2. **Escape `#xD` as `&#xD;`** so a literal CR survives a round-trip instead of
   being normalized to `#xA` by the reader.
3. **Refuse the C0 scalars XML's `Char` production excludes, naming the scalar.**
   U+0000 is not in `Char` at all, so no escape rescues it; the serializer must
   hard-error rather than emit an unreadable document. The diagnostic must name
   the offending scalar (current draft: `U+0000 at byte N is not permitted in XML 1.0`).
4. **Verification is writer-then-reader, not a unit test.** Write with the
   production writer, read back with the production reader, assert the graph is
   identical. This is stated as "the only check that would have caught either
   defect."
5. **Any frozen vector or conformance row that moves gets explained, never
   absorbed.** Byte movement must be reviewed as byte movement.

### Implicit Requirements

1. **Both writers must be fixed**, not just RDF/XML: "The same loss occurs through
   TriX," and the defect statement covers both files.
2. **Attribute context is in scope.** XML §3.3.3 also normalizes attribute tabs
   and line feeds, so `#x9`, `#xA`, and `#xD` must be emitted as character
   references inside attribute values. The issue's example is character data, but
   the shared law must be context-aware (Text vs Attribute).
3. **Hard error, not silent degradation** (AGENTS.md §H / ETHOS §H). Today's
   exit-zero-with-corrupt-output must become a non-zero, actionable failure.
4. **Byte determinism preserved.** Escaping must remain deterministic and
   order-independent; no time/RNG/iteration-order dependence may enter the output
   path. Changing bytes for CR-bearing literals is expected and must be justified
   in the PR.
5. **No refusal regression ("over-refusal watch").** The new C0 refusal must not
   over-refuse valid neighbours: `#x9`, `#xA`, `#xD`, U+0085, U+00A0, and the
   astral planes must all keep round-tripping.
6. **No scope creep into, or regression of, the scanner-terminal work.** Both
   defects pre-date that branch and neither is reachable from any of its
   acceptance criteria; that work's terminal law must remain intact.
7. **The `Char` predicate is the enumerated production, not a Unicode property**
   (AGENTS.md terminal ring-fence). The compliant implementation is one shared
   predicate with its production cited (`purrdf_iri::terminals::is_xml_char`),
   not an ad-hoc "C0 means `< 0x20`" check.
8. **Wasm-able and dependency-clean.** `purrdf-core`/`purrdf-rdf` constraints
   hold; no new dependency may drag in platform resources.
9. **SPDX headers** on any new source file (`MIT OR Apache-2.0`).

---

## Acceptance Criteria

Derived from the issue's "What closing this looks like" plus its stated defect
examples and over-refusal watch:

- [ ] Exactly one shared XML-escape implementation exists on the egress path; no
      byte-identical copy remains in `rdfxml.rs` and `trix.rs`.
- [ ] A literal containing a CR (`"a<CR>b"`) round-trips through **RDF/XML**
      (ntriples → rdfxml → ntriples) to the *same* literal, not `"a\nb"`.
- [ ] The same CR round-trip holds for **TriX**.
- [ ] A literal containing a scalar outside `Char` (canonical case U+0000) makes
      the writer **fail with a diagnostic naming the scalar**, with non-zero
      exit; no document is emitted at exit zero.
- [ ] The emitted diagnostic identifies the scalar (e.g. `U+0000`) and, ideally,
      its byte offset.
- [ ] Positive neighbours keep working, executed as tests/round-trips:
      `#x9`, `#xA`, `#xD`, U+0085, U+00A0, and astral-plane scalars.
- [ ] Attribute-context normalization is covered: `#x9`/`#xA`/`#xD` inside
      attribute values survive (are emitted as references, not raw).
- [ ] Integration verification uses the **production writer then the production
      reader**, asserting graph identity/isomorphism — not a unit test on the
      escape function alone. (The round-trip harness already exists in
      `trix.rs` tests and `crates/rdf/tests/xml_character_roundtrip.rs` in the
      working-tree draft; ensure it covers both defects.)
- [ ] Every frozen vector or conformance row whose bytes move is listed and
      explained in the PR; none regenerated silently.
- [ ] `make check` passes: fmt, clippy (pedantic + nursery, warning-free),
      workspace tests, and the wasm build.
- [ ] New source files carry SPDX headers.

---

## Dependencies

### Depends On

- **None explicitly stated.** The issue names no `#X` blocker. It references the
  *scanner-terminal work* only to explain why this was filed separately.

### Blocks

- (none declared)

### Related Issues

- **Unnamed scanner-terminal issue/work** (the fifth-to-seventh escape-transcription
  collapse): context only. This issue is the sixth and seventh transcription and
  the same shape as that work's headline, but is deliberately split out because it
  moves bytes and adds a hard error. It must not be folded back into that work.

### Internal / Code Dependencies

- `purrdf-rdf` → `purrdf-core`: the shared egress home lives in `rdf-core`
  (issue names `crates/rdf-core/src/iri_escape.rs`; working tree has
  `crates/rdf-core/src/xml_escape.rs`).
- `purrdf_iri::terminals::is_xml_char`: the single spelled-once XML `Char`
  production (ranges asserted at compile time) that the refusal must call.
- `purrdf` diagnostics: writers already return `Result<_, RdfDiagnostic>`; the
  escape error must plumb through that path.
- Frozen GTS vectors in `vectors/` (governed in `gmeow-gts`) and the W3C/ShexTest
  conformance corpora — consumers of the bytes this change may move.

---

## Context from Discussion

The staged file contains **0 comments**, so there are no recorded decisions,
rejected approaches, or open questions from discussion. All context is from the
body.

### Key Decisions (from the issue body)

- Filed rather than fixed in place, because the change moves emitted bytes and
  converts exit-zero-corrupt into a hard error; it belongs in a byte-diff PR
  reviewed as such.
- The `#xD` case is a *lossy round-trip* (silent corruption); the C0 case is a
  *non-well-formed emission* (silent corruption + unreadable output). Both are
  treated as in scope.
- Verification is explicitly designated as writer→reader, not unit-level.

### Rejected Approaches

- **Riding along inside the scanner-terminal change** — rejected: different
  review lens (character classes vs byte movement), and neither defect is
  reachable from that work's acceptance criteria.
- **Implicitly rejected: escaping U+0000.** The issue states no escape rescues it
  ("U+0000 is not in XML's `Char` production at all"), so the required behavior
  is refusal, not a character reference.

### Open Questions

- Which is the canonical shared home: the existing
  `crates/rdf-core/src/iri_escape.rs` named by the issue, or a new dedicated
  `xml_escape.rs`? (The draft working tree chose the latter. Needs a plan
  decision consistent with "one shared home" and with `iri_escape`'s current
  responsibilities.)
- Scope: the issue names exactly two writers. The working tree's draft also
  touches `crates/rdf/src/projections/util.rs`, `crates/rdf/src/viz/svg.rs`, and
  `crates/sparql-results/src/xml.rs`. Are those in scope as fellow consumers of
  the shared law, or out of scope for this issue? The issue text does not say.
- Error surface: is the refusal a new dedicated error type carried by
  `RdfDiagnostic`, and does it abort atomically (no partial output) — the draft
  `push` claims atomicity, but the plan should state it.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Emitted bytes change for CR-bearing literals → frozen vectors / conformance goldens move | HIGH (for affected docs) | MED | Enumerate every moved golden; explain each in the PR; never silently regenerate. Keep the change isolated from unrelated work. |
| New C0 hard error over-refuses valid neighbours (regression) | MED | HIGH | Executed positive round-trips for `#x9`, `#xA`, `#xD`, U+0085, U+00A0, astral planes ("over-refusal watch"). |
| `Char` predicate wrongly approximated (e.g. "`< 0x20`" or a Unicode property) silently accepts/rejects wrong scalars | MED | HIGH | Use the single `purrdf_iri::terminals::is_xml_char` with its production cited; keep terminal-hygiene gate green. |
| Text vs attribute context conflated; tabs/LFs in attributes normalize away | MED | MED | Context-aware escape (`Text` / `Attribute`) with explicit tests for both. |
| Duplication re-appears / only one writer fixed | MED | MED | One shared function; delete both local copies; a test that exercises both writers through the reader. |
| Error plumbing produces partial output before failing | LOW | MED | Validate-then-write / atomic buffer semantics; test that output is unchanged on failure. |
| Verification degrades to a unit test on the escape fn (what the issue explicitly forbids) | MED | MED | Require production-writer→production-reader integration asserting graph identity. |
| Byte-determinism regression (order/RNG dependence introduced) | LOW | HIGH | Keep escape pure and deterministic; goldens + deterministic-emission tests. |
| Scope uncertainty (extra XML-shaped writers) causes either under- or over-reach | MED | LOW | Settle scope in planning; if other writers share the law, fix them in this change or file explicitly, but do not silently absorb byte movement. |
| Conformance harness XPASS/xfail discipline trips when rows flip | MED | MED | Run the conformance suites; update ledgers with reasons; exact-count assertions must stay honest. |

**Complexity:** Low algorithmic complexity, **high blast radius and verification
cost.** The code change is small (a shared helper plus two call sites plus error
plumbing); the real work is (a) proving valid neighbours still pass, (b) auditing
and explaining every moved frozen vector / conformance row, and (c) doing it with
production writer→reader round-trips rather than unit tests.

---

## Technical Notes

### Affected Areas

- `crates/rdf/src/native_codecs/rdfxml.rs` — RDF/XML writer. Issue cites the
  duplicated escape at `:1190`; in the checked-out tree the escaping delegates
  via `escape_xml_text` (`:1181`) and `escape_xml_attr` (`:1187`); call sites
  include character data (`:1116`, `:1574`) and attribute values (`:1129`,
  `:1131`, `:1139`, `:1287`, `:1294`, `:1388`, `:1395`, `:1549`, `:1556`,
  `:1562`, `:1571`). RDF/XML is a `Char`-sensitive surface plus `parseType`
  handling.
- `crates/rdf/src/native_codecs/trix.rs` — TriX writer. Issue cites the
  duplicate at `:550`; checked-out tree has `escape_text` (`:536`) and
  `escape_attr` (`:542`), called for `uri`/`id`/plain literals (`:467`, `:470`,
  `:486`, `:489`, `:502`, `:507`, `:514`).
- **Shared egress home:** issue names `crates/rdf-core/src/iri_escape.rs`;
  working tree draft introduces `crates/rdf-core/src/xml_escape.rs` containing
  `Context { Text, Attribute }`, `InvalidXmlChar`, `replacement`, `escape`,
  and `push`.
- `crates/iri/src/terminals.rs` — `is_xml_char` (`:512`–`:517`): the enumerated
  XML 1.0 §2.2 `[2] Char` production; ascii `0x09–0x0A`, `0x0D`, `0x20–0x7F`;
  non-ASCII `0x80–0xD7FF`, `0xE000–0xFFFD`, `0x10000–0x10FFFF`. This is the
  predicate the refusal must route through.
- **Adjacent XML-shaped egress surfaces** (scope decision needed):
  `crates/rdf/src/projections/util.rs`, `crates/rdf/src/viz/svg.rs`,
  `crates/sparql-results/src/xml.rs` (all touched by the working-tree draft).
- **Tests:** `crates/rdf/tests/xml_character_roundtrip.rs` (draft), TriX
  in-module round-trip harness (`trix.rs:547` sqq.), and
  `crates/sparql-results/tests/xml_characters.rs` (draft). The issue requires
  writer→reader isomorphism, not unit tests.
- Frozen corpora that may move: `vectors/` (GTS, governed in `gmeow-gts`),
  `crates/rdf/tests/fixtures/rdfc/`, W3C SHACL suite, shexTest, first-party
  SHACL corpus, SPARQL conformance.

### Potential Challenges

- **XML normalization rules are split across spec sections:** §2.11 (character
  data CR → LF) and §3.3.3 (attribute tab/LF/CR → space). A single context-free
  escaper cannot be correct; context must be threaded through.
- **`Char` exclusion set is subtle:** C0 except `#x9`, `#xA`, `#xD`; surrogates
  `#xD800–#xDFFF` (unreachable in Rust `char`); and `#xFFFE`/`#xFFFF` are
  excluded, as is anything above `#x10FFFF`. Getting this from one shared
  predicate, not from a local heuristic, is the whole point.
- **`Rust char` cannot hold surrogates**, but it can hold C0 and `#xFFFE/#xFFFF`;
  the refusal must cover those.
- **API/error plumbing:** the escape error must become a `serialize_err`-style
  `RdfDiagnostic` with a scalar-naming message, and must not leave partial output.
- **Byte determinism and goldens:** any new character reference changes bytes for
  affected documents only; proving "only affected documents moved" is part of the
  review, not an assumption.

### Current Repository State (observed — verify against committed base)

The worktree at `.worktrees/294-xml-writers-emit-literals-they-cannot` is on
branch `paudley/294-xml-writers-emit-literals-they-cannot` at base `ed03dcd1`
(== `origin/main`), **with a dirty working tree** containing an uncommitted draft
that appears to already implement much of this issue:

```
 M crates/iri/src/terminals.rs
 M crates/rdf-core/src/blank_label.rs
 M crates/rdf-core/src/lib.rs
 M crates/rdf/src/native_codecs/rdfxml.rs
 M crates/rdf/src/native_codecs/trix.rs
 M crates/rdf/src/projections/util.rs
 M crates/rdf/src/viz/svg.rs
 M crates/sparql-results/src/xml.rs
?? crates/rdf-core/src/xml_escape.rs
?? crates/rdf/tests/xml_character_roundtrip.rs
?? crates/sparql-results/tests/xml_characters.rs
```

Consequences for planning:

1. The issue's cited duplication line numbers (`rdfxml.rs:1190`, `trix.rs:550`)
   no longer match the working tree; the quoted premise ("transcribed twice") may
   already be addressed in the uncommitted draft.
2. Confirm whether the dirty tree is prior work product to be reviewed/finished,
   or pre-existing state. Do not assume the issue's defects are unfixed without
   checking the committed base and the draft diff.
3. The draft chooses a new `xml_escape.rs` rather than the issue-named
   `iri_escape.rs`; reconcile with the "one shared home" requirement.

---

## What "Closing This Looks Like" (per the issue, verbatim intent)

- One `escape_xml`, not two. The workspace already has a shared home for egress
  escaping in `crates/rdf-core/src/iri_escape.rs`.
- Escape `#xD` as `&#xD;` so a literal CR survives a round-trip.
- Refuse the C0 scalars XML's `Char` production excludes, naming the scalar,
  rather than emitting a document that cannot be read.
- Verification is **writer then reader**, not a unit test: write with the
  production writer, read back with the production reader, assert the graph is
  identical. That is the only check that would have caught either defect.
- Any frozen vector or conformance row that moves gets explained, never absorbed.

**Over-refusal watch (executed valid neighbours):** `#x9`, `#xA`, and `#xD` are
all in XML's `Char` production and must keep round-tripping, as must U+0085,
U+00A0, and the astral planes. Both defects pre-date the branch that found them,
and neither is reachable from any acceptance criterion of the scanner-terminal
work.

---

## Related PRs

- None present in the staged file. (No PR links were provided and none were
  fetched.)
