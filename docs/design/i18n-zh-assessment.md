<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Assessment: translating the PurRDF documentation into Simplified Chinese for 0.14.0

**Status:** assessment only — nothing in this document is implemented.
**Target:** Simplified Chinese, BCP 47 `zh-Hans`, for mainland-China technology
and AI developers. Traditional Chinese (`zh-Hant`) is out of scope for 0.14.0
and is not assessed here.
**Voice reference:** <https://blackcatinformatics.ca/zh/> (and its
`/zh/purrdf/` page), cited throughout as "the house page".
**Method:** every count below was measured with `wc -w`, `grep`, or a short
Python pass over the tree at the commit this document was written against;
every gate finding was reproduced by running the gate's own code on Chinese
input. Nothing is estimated.

## 1. Summary and recommendation

PurRDF has about **40,600 words of book, 33,700 words of README, and 68,700
words of `docs/*.md`** in English, of which 74–95 % per surface is translatable
prose (the rest is code, IRIs, identifiers and keywords that stay verbatim).
Beneath that sits a **runtime-string tier** — 6,300 words of CLI `--help`, 139
diagnostic construction sites over 79 stable codes, 5,200 words of `.pyi`
comment text plus 19,300 words of PyO3 rustdoc that become Python `__doc__`,
and 4,100 words of hand-written JSDoc — and beneath that **873,000 words of
rustdoc** that docs.rs cannot serve in any language but the one it was written
in.

The difficulty is not the volume. It is that this repository **gates prose
mechanically**, and every one of those gates was written against English:

* `check-brand-casing.py` treats a CJK character as an identifier character
  (`'本'.isalnum()` is `True`), so lowercase `purrdf` glued to Chinese text is
  silently classified as "part of an identifier" and never flagged. The house
  typography (a half-width space between Latin and CJK) happens to keep the
  gate working; a translator who drops the space defeats it without noticing.
* Five of the process-token families in `check-issue-refs.py` are anchored on
  `\b`, which Python does not place between a Latin letter and a CJK character,
  so those families go blind when glued to Chinese. `#NNN` itself is still
  caught.
* `check-spec-attribution.py` scans whole-file text including code blocks, and
  its disclaimer list is English-only, so a Chinese page that shows the
  quad-template example (a first-party extension, not defined by SPARQL 1.2)
  near the words "SPARQL 1.2" with a *Chinese* disclaimer **fails** the gate.
* `check-doc-claims.py` names 105 claims by English sentence in named files: a
  translation inherits none of them, and its sentence splitter
  (`(?<=[.!?])\s+`) does not recognise `。！？`, so a Chinese paragraph is one
  sentence to the overclaim ban.
* `serializer_roundtrip_sweep.rs` and `shipped_sparql_examples.rs` extract
  every fenced `sparql` block under `docs/**` — 24 today, 22 of them in one
  chapter — and cap the unparseable count at 122. A parallel Chinese tree
  doubles the swept set; a gettext `.po` file is invisible to both tests and to
  all four scripts above, because no gate scans `.po`.
* mdBook's search (elasticlunr, `/[\s\-]+/` tokenizer, English pipeline)
  indexes **zero CJK tokens**. Reproduced on both the locally installed 0.4.52
  and the CI-pinned 0.5.3: a Chinese heading produces an empty `title` index.
  A Chinese reader searching in Chinese gets "no results" — a silent failure
  that looks like the book simply lacks the topic.

**Recommendation.**

1. **Translate the book with `mdbook-i18n-helpers` (gettext `.po`), not a
   parallel tree.** Its untranslated-falls-back-to-English behaviour is the
   only mechanism on the table that makes translation *lag visible to the
   reader* by construction; a parallel tree makes a stale page read as current,
   which is the silent-success failure this repository is organised to
   prevent. The tool must be pinned exactly like `GIT_CLIFF_VERSION` and
   `MDBOOK_VERSION`, and a new gate must render the translated Markdown and run
   the existing four scripts over it, because otherwise the translation is
   ungated.
2. **Fix the two gate blind spots before the first Chinese page lands**
   (`isalnum` → ASCII-only in `check-brand-casing.py`; `\b` → explicit
   ASCII lookarounds in `check-issue-refs.py`; a Chinese disclaimer set in
   `check-spec-attribution.py`). Each is small, and each is a tightening, so
   each needs the paired valid-neighbour case proven per `CLAUDE.md`.
3. **Ship a glossary file as a gate input, not a wiki page.** The 30-term
   sample in §5 shows that roughly a third of PurRDF's load-bearing terms have
   no established mainland rendering. The house style already settles the
   policy: spec names and acronyms stay English, concepts with a settled
   knowledge-graph rendering use it, and coined terms carry the English term
   on first use.
4. **Phase by audience value, not by chapter order** (§7): README and
   introduction, the Python entry path, then the SPARQL extension and
   retrieval surfaces (composite datatypes, statistical aggregates, embedding
   kNN, full-text scoring), then the remainder of the book.
5. **Do not localise runtime strings for 0.14.0.** CLI help, diagnostics and
   Python `__doc__` need a locale mechanism that does not exist in the code,
   diagnostics are pinned by tests, and Python has no per-language docstring
   convention. That tier is a multi-release project (§7, phase 5). What *can*
   ship is a Chinese **reference page for the 79 diagnostic codes**, which is
   documentation, not code.
6. **Rustdoc is not translated.** docs.rs has no i18n; 873,000 words is larger
   than every other tier combined; the target audience reads English API
   references routinely.
7. **Serve the Chinese book from `blackcatinformatics.cn`** as well as GitHub
   Pages, and add one paragraph to the install pages naming the standard
   mainland mirrors (§6). This is the cheapest single improvement for the
   stated audience.

Overall magnitude: the book-plus-README translation is **large** (comparable to
a mid-sized crate), the gate and tooling work is **medium**, the Python API
guide is **large**, and the runtime-string tier is **multi-release**.

## 2. Inventory (measured)

### 2.1 Documentation surfaces

| Surface | Files | Words (`wc -w`) | Fenced code | Inline code | Prose (upper bound) | Prose % |
|---|---:|---:|---:|---:|---:|---:|
| `README.md` (root) | 1 | 2,744 | 199 | 155 | 2,460 | 89 % |
| `crates/*/README.md` | 24 | 25,150 | 2,815 | 1,902 | 20,873 | 82 % |
| `bindings/python/README.md` | 1 | 3,858 | 488 | 342 | 3,114 | 80 % |
| `bindings/python/tests/README.md` | 1 | 440 | — | — | — | (contributor) |
| `crates/rdf-wasm/js/README.md` (npm) | 1 | 1,484 | 277 | 158 | 1,108 | 74 % |
| `docs/book/src/**` (mdBook) | 29 | 40,556 | 3,367 | 2,873 | 35,157 | 86 % |
| `docs/*.md` | 13 | 68,679 | 4,407 | 4,097 | 61,292 | 89 % |
| `docs/design/*.md` | 4 | 13,466 | 74 | 746 | 12,857 | 95 % |

"Prose" here is words outside fenced blocks and inline-code spans, minus
tokens that match an IRI, a prefixed name, or a SPARQL keyword. It is an upper
bound: table cells and headings full of identifiers still count as prose, so
the true translatable fraction is a few points lower on the reference-heavy
pages (`crates/cli/README.md`, `docs/GTS-SPEC.md`).

The 24 crate READMEs are dominated by five: `cli` 7,317, `shapes` 3,517,
`rdf-capi` 1,742, `entail` 1,627, `rdf` 1,539. The audience-priority crate
READMEs are small: `text` 778, `geo` 546, `cdt` 274, `purrdf` 1,181.

### 2.2 The book, per chapter

| Chapter | Words | Prose % | Audience priority (§7) |
|---|---:|---:|---|
| `sparql/querying.md` | 13,519 | 83 % | high — composite datatypes, statistical aggregates, path witnesses, extension hosts |
| `entailment.md` | 3,852 | 92 % | medium |
| `concepts/projections.md` | 3,241 | 89 % | medium |
| `validation/shacl.md` | 2,369 | 92 % | medium |
| `concepts/base-iris.md` | 2,365 | 85 % | low |
| `entailment-rules.md` | 2,155 | 92 % | low — **generated** (see §3.7) |
| `concepts/codecs.md` | 1,390 | 92 % | high — holds "Deterministic embedding companions" |
| `project/conformance.md` | 783 | 89 % | low — restates gated numbers |
| `introduction.md` | 753 | 98 % | highest |
| `project/design-rules.md` | 733 | 97 % | low |
| `sparql/results.md` | 731 | 83 % | medium |
| `getting-started/c.md` | 653 | 72 % | low |
| `getting-started/python.md` | 596 | 81 % | highest |
| `datalog.md` | 580 | 93 % | medium |
| `gts.md` | 575 | 87 % | medium |
| `getting-started/javascript.md` | 544 | 68 % | high |
| `project/releases.md` | 544 | 80 % | low |
| `slices.md` | 542 | 89 % | low |
| `concepts/rdf12.md` | 531 | 94 % | high |
| `interop/rdfjs.md` | 527 | 70 % | medium |
| `concepts/interned-dataset.md` | 519 | 86 % | high |
| `concepts/visualization.md` | 465 | 98 % | low |
| `getting-started/rust.md` | 452 | 69 % | high |
| `project/performance.md` | 434 | 85 % | low |
| `concepts/jsonld.md` | 432 | 69 % | medium |
| `interop/rdflib.md` | 399 | 88 % | high (Python audience) |
| `validation/shex.md` | 386 | 84 % | low |
| `concepts/canonicalization.md` | 357 | 89 % | medium |
| `SUMMARY.md` | 129 | — | structure only; chapter titles are translatable strings |

`SUMMARY.md` has seven parts (Getting Started, Concepts, SPARQL, Validation,
Reasoning & Transport, Interop, Project) and one nested entry
(`entailment-rules.md` under `entailment.md`). `book.toml` declares
`language = "en"`, no preprocessors, no theme directory, and an
`edit-url-template` pointing at `docs/book/src/{path}`.

### 2.3 `docs/*.md` and `docs/design/*.md`, classified

| File | Words | Classification | Translate? |
|---|---:|---|---|
| `GTS-SPEC.md` | 21,175 | normative wire-format specification; governed in `gmeow-gts` | no — a translated normative spec is a separate governance decision, not a docs task |
| `PURREMB.md` | 11,476 | binary-format specification for embedding companions (23 sections) | partially — scope, reader model, and the SPARQL-facing sections; the byte-layout sections stay English |
| `CONFORMANCE.md` | 9,587 | user-facing scoreboard, machine-written block plus gated prose | **no** — link to it; see §3.4 |
| `SPARQL-GOVERNOR-PROFILE.md` | 7,025 | user-facing profile (governors, budgets, refusals) | later phase; heavy coinage load |
| `BENCHMARKS.md` | 5,969 | report of measurements | no — numbers, report-only |
| `GTS-CONFORMANCE.md` | 3,495 | implementer-facing | no |
| `RELEASE.md` | 2,475 | contributor-facing | no |
| `RDF12-CANON-PROFILE.md` | 2,012 | user-facing profile | later phase |
| `GTS-THIRD-PARTY-IMPLEMENTER-GUIDE.md` | 1,490 | implementer-facing | no |
| `CUTOVER.md` | 1,376 | contributor-facing (downstream migration) | no |
| `NPM-ECOSYSTEM.md` | 1,289 | generated probe report | no |
| `COLUMNAR.md` | 867 | user-facing schema note | later phase |
| `BRAND.md` | 443 | contributor-facing | no — but its rule binds the translation (§5.1) |
| `design/purrdf-text-scoring.md` | 3,315 | design rationale for full-text scoring | yes for this audience — phase 3 |
| `design/purrdf-embedding-knn.md` | 2,385 | design rationale for embedding kNN | yes for this audience — phase 3 |
| `design/purrdf-backend-contract.md` | 4,804 | contributor-facing | no |
| `design/purrdf-geo-exactness.md` | 2,962 | design rationale | later phase |

Root `AGENTS.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `PROVENANCE.md`,
`LICENSING.md` are contributor- or agent-facing and are not translated.

### 2.4 Runtime strings that users read (code, not docs)

| Surface | Measure | Mechanism today |
|---|---|---|
| CLI `--help` | `crates/cli/src/cli.rs`: 6,326 words of `///` doc comments across 110 `#[arg]`/`#[command]` sites and 77 `value_name`s; 36,356 rustdoc words in the crate overall | clap derives help from rustdoc; clap has no locale mechanism |
| Diagnostics | `RdfDiagnostic { severity, code, message, detail, location }`; 139 constructor call sites (`RdfDiagnostic::new/error/warning`), 79 distinct stable code strings (families: `owl-*`, `iri-*`, `gts-*`, `rdf-*`, `jsonld-*`, `sparql-*`, `reasoning-*`, `cdt-*`, `text-*`, `datalog-*`); 269 `write!(f, "…")` `Display` sites | `message` is a `String` built at the site; tests match on `code` and, in goldens, on `message` |
| Python | `bindings/python/python/src/purrdf/__init__.pyi`: 10,248 words, of which 5,171 are `#`-comment documentation and 26 are docstrings; 157 `#[pyclass]`/`#[pyfunction]`/`#[pymethods]` sites carrying 19,336 rustdoc words that PyO3 emits as `__doc__`; 480 docstrings (7,445 words) in the pure-Python layer | one `__doc__` string per object; one `.pyi` per package; no per-language convention in Python, pyright or mypy |
| JavaScript | `crates/rdf-wasm/js/index.d.ts`: 6,959 words, 97 JSDoc blocks (4,051 words); `index.mjs` 3,049 words; 12,931 rustdoc words in `crates/rdf-wasm/src` that `wasm-bindgen` copies into the generated `.d.ts` | one `.d.ts`; TypeScript has no locale-selected typings |

This tier is **code**. Translating any of it means adding a locale mechanism,
touching test goldens, and accepting that a Chinese `message` string diverges
from the English one that every downstream (`gmeow-ontology`, the playground,
the SARIF emitter) matches on. That is a different and much larger project
than translating the book (§7, phase 5).

### 2.5 Rustdoc

873,106 words of `///` and `//!` across `crates/*/src` (`entail` 181,137;
`sparql-eval` 159,433; `rdf-core` 79,663; `rdf` 59,468; `datalog` 47,644;
`shapes` 45,341) plus 21,970 in `bindings/python/src`. The count includes
doc-example code lines, so the prose share is lower, but even at half it dwarfs
every other tier. docs.rs renders exactly one language; the tier is not
translated.

### 2.6 What must not be translated

Invariant in every language: the brand **PurRDF** (the house page confirms
this — it writes `PurRDF`, `GMEOW`, `GTS` untranslated inside Chinese
sentences); every IRI and prefixed name; SPARQL, Turtle, ShEx and SHACL
keywords; diagnostic code strings (`iri-relative-no-base`); crate, module and
API names; CLI flags and `value_name`s; file paths; spec section numbers
(`SEP-0009`, `RDFC-1.0`); profile identifiers (`purrdf-rdfc12`); and the
fenced blocks themselves. Measured: fenced code is 8 % of the book and 11 % of
the crate READMEs; inline code is a further 7 % of each; IRIs and keywords
outside code are under 1 %. Fence census across the documentation: `sh` 79,
`text` 57, `rust` 43, `python` 29, `bash` 25, `sparql` 24, `js` 16, `cddl` 12,
`json` 10. The 24 `sparql` fences sit in two files: 22 in
`docs/book/src/sparql/querying.md`, 2 in `docs/design/purrdf-text-scoring.md`.

## 3. Gate-by-gate findings

### 3.1 `scripts/check-brand-casing.py`

The bare-word test is:

```python
_ADJACENT_IDENTIFIER_CHARS = frozenset("-_:/.`")

def is_bare(text: str, start: int, end: int) -> bool:
    before = text[start - 1] if start > 0 else " "
    after = text[end] if end < len(text) else " "
    if before in _ADJACENT_IDENTIFIER_CHARS or before.isalnum():
        return False
    if after in _ADJACENT_IDENTIFIER_CHARS or after.isalnum():
        return False
    return True
```

`str.isalnum()` is Unicode-aware: `'本'.isalnum()` is `True`. Running
`scan_markdown` on six one-line files:

| Case | Input line (shown as code so this table is itself gate-clean) | Hits |
|---|---|---:|
| A — house typography (half-width space) | `本工具包名为 purrdf ，用于处理 RDF 1.2。` | 1 |
| B — glued to CJK on both sides | `本工具包名为purrdf，用于处理RDF。` | **0** |
| C — full-width punctuation after | `purrdf。它是一个工具包。` | 1 |
| D — capitalised, glued | `PurRDF工具包支持SPARQL。` | 0 (correct: `PurRDF` is never flagged) |
| E — English control | `the purrdf toolkit` | 1 |
| F — crate name in backticks | `` `purrdf` 门面 crate `` | 0 (correct) |

Case B is the defect: glued lowercase `purrdf` is classified as an identifier
fragment (as if it were `libpurrdf`) and passes silently. The house page
always puts a half-width space between Latin and CJK runs, so a translator who
follows the typography keeps the gate honest — but the gate is then relying on
a style rule it does not check. The fix is one clause
(`before.isascii() and before.isalnum()`), and because it is a tightening it
must ship with both proofs: case B flagged, and `libpurrdf`/`purrdfs` still
not flagged.

Three scope facts matter more than the regex. First, the scan covers `.rs` and
`.md` only (`iter_scan_paths`): a gettext `.po` file is **outside the gate
entirely**. Second, enumeration is `git ls-files`, so an **untracked** file is
not scanned at all — a translator who runs the gate before `git add` gets a
vacuous pass (this assessment tripped over exactly that: the report passed
locally while untracked and failed in CI once tracked). The same is true of
`check-issue-refs.py`; `check-spec-attribution.py` and `check-doc-claims.py`
walk the filesystem and do see untracked files. Third, `PRE_EXISTING_BRAND_CASING` is a frozen per-file count
register; a new Chinese file with any bare `purrdf` is an unregistered
offender and simply fails, which is the right behaviour for new text.

The gate runs in `make check` but **not in `ci.yaml`** (the CI job runs
`check-issue-refs.py` and `check-spec-attribution.py` directly; brand casing is
local-only today). Any translation gate wired to it inherits that gap.

### 3.2 `scripts/check-issue-refs.py`

```python
ISSUE_PATTERN = r"#\d{1,5}(?![\dA-Fa-f-])(?!\.\d)"
```

The tracker-item family has no leading `\b`, so `参见#NNN。` (with digits) is
caught with or without spaces — verified. The process-token families are
`\b`-anchored (`\b[Tt]ask\s+#?\d+\b`, `\bEPIC\b`, `\bH\d{1,3}\b`,
`\bAC\d\b`, and the two phrases that locate text in repository history). Python places
`\b` only between `\w` and `\W`, and CJK characters are `\w`, so:

```
re.search(r'\bH\d{1,3}\b', '风险H12点')   -> False
re.search(r'\bH\d{1,3}\b', '风险 H12 点') -> True
```

Glued Chinese defeats every `\b`-anchored family; the house spacing rescues
them in practice. The label families that end in a colon (a letter, one to
three digits, then a colon) still match because the colon is the boundary. Fix: replace the `\b`s with explicit
ASCII lookarounds `(?<![A-Za-z0-9_])` / `(?![A-Za-z0-9_])`. The gate scans
`.rs .md .toml .py .pyi .yaml .yml .ttl .nt .nq .rq` — again not `.po`.

None of the patterns assume English *sentence* structure; they are token
patterns, and a Chinese sentence carrying an English development-process token
is exactly as wrong as an English one, so the gate's intent transfers cleanly
once the boundary fix lands.

### 3.3 `scripts/check-spec-attribution.py`

```python
WINDOW = 220
ANCHOR_ALTERNATION = (
    r"quad[\s-]?templates?|CONSTRUCT\s*\{\s*GRAPH"
    r"|CONSTRUCT\s+GRAPH\b|CONSTRUCT\s+templates?\b"
)
SPEC_RE = re.compile(r"SPARQL\s*-?\s*1\.2", re.IGNORECASE)
DISCLAIMERS: tuple[str, ...] = (
    "not defined by sparql",
    "not a sparql 1.2 feature",
    ...
    "purrdf extension",
    "neither sparql 1.1 nor sparql 1.2",
    ...
)
```

Three consequences for a Chinese page:

1. The anchor is matched in **whole-file text including fenced code**
   (the docstring says so and the scan confirms it), so the code example
   `CONSTRUCT { GRAPH …` on a translated querying page is an anchor even
   though the prose around it is Chinese.
2. `SPARQL 1.2` is retained in Chinese (house style), so the spec token
   matches.
3. Every `DISCLAIMER` is English. A Chinese sentence such as
   「四元组模板并非 SPARQL 1.2 特性，而是 PurRDF 的扩展」 contains no
   disclaimer the gate recognises. **The translated page fails the gate as a
   false positive.** The remedy is to extend `DISCLAIMERS` with the agreed
   Chinese wordings (e.g. `并非 SPARQL 1.2`, `不是 SPARQL 1.2 特性`,
   `PurRDF 扩展`, `SPARQL 1.2 并未定义`) and add them to `--self-test`.

`WINDOW = 220` is in characters. A 220-character Chinese window carries two to
three times the information of a 220-character English one, so the window is
effectively wider in Chinese; that only makes a missing disclaimer more likely
to be caught, not less. This gate is the one that is *over*-strict on Chinese,
and it is in `ci.yaml`.

### 3.4 `scripts/check-doc-claims.py` (105 gated numeric claims)

Running it on this tree: `OK: 105 documented claim(s) agree with their generated
source (stale-name ban swept 1031 file(s); entailment-overclaim ban swept 27 of
134 prose unit(s) …)`.

The 105 claims are located by **English sentence text in named files**:

```python
_CONFORMANCE = _REPO / "docs" / "CONFORMANCE.md"
_ENTAILMENT = _REPO / "docs" / "book" / "src" / "entailment.md"
_BOOK_CONFORMANCE = _REPO / "docs" / "book" / "src" / "project" / "conformance.md"
_README = _REPO / "README.md"
_ENTAIL_README = _REPO / "crates" / "entail" / "README.md"
_PY_README = _REPO / "bindings" / "python" / "README.md"
```

A translated `docs/CONFORMANCE.md` therefore inherits **zero** of its claims: the
numbers would be duplicated with no guard. The two honest options are (a) extend
the gate with a second, Chinese sentence for every claim — 105 more anchors
that go stale whenever the Chinese wording changes — or (b) **do not translate
the scoreboard**; keep one `CONFORMANCE.md`, translate only the surrounding
explanation, and link. This document recommends (b). The same reasoning applies
to `project/conformance.md` and the coverage tables in `entailment.md` and
`crates/entail/README.md`: the numbers stay in the English source and the
translation links to them, or the translation restates them and the gate learns
the second sentence. The gettext route makes this concrete: a number inside a
`msgstr` is retranslated when the `msgid` changes (the entry becomes fuzzy and
falls back to English), so the drift window is bounded by the fuzzy check
rather than by memory.

Two of its sweeps *do* reach a translation automatically, because they walk
every `.md` under `docs/`:

```python
for root in ("crates", "bindings", "docs"):
    for path in sorted((_REPO / root).rglob("*")):
        if path.suffix not in {".rs", ".md", ".pyi", ".mjs", ".ts"}:
            continue
```

The entailment-overclaim ban reads the reflowed text sentence by sentence with

```python
_TERMINATOR = re.compile(r"(?<=[.!?])\s+")
```

which splits `'A sentence. Another one!'` into two but leaves
`'第一句。第二句！第三句？第四句。'` as **one sentence**. Consequences: a
scope phrase anywhere in a Chinese paragraph exempts every claim in that
paragraph (weakened, not broken); the ASCII subject marker `owl 2` still fires
because `OWL 2` is retained in Chinese; the English adjective in the fourth
ban row never fires because it is translated (「完全符合」 is invisible to it).
A `.po` file is outside this sweep as well. Adding `。！？` to `_TERMINATOR`
is a one-line change and should ship with the gate fixes in §3.1–3.3.

### 3.5 `crates/sparql-algebra/tests/serializer_roundtrip_sweep.rs`

```rust
const MAX_UNPARSEABLE_RQ: usize = 122;
...
let doc_examples = collect_doc_examples(&root.join("docs"));
...
const N: usize = 652 + 171;
assert!(seen >= N, "...");
...
assert!(unparseable <= MAX_UNPARSEABLE_RQ, "...");
```

`collect_doc_examples` walks every `.md` under `docs/` and takes every fenced
block whose opener is ` ```sparql `, whole. The count assertion is a **floor**
(`seen >= 823`), so a parallel Chinese tree that doubles the 24 blocks to 48
raises `seen` harmlessly. The ceiling is the risk: every block a translator
breaks counts against `122`, and the message a maintainer then reads says "a
parser regression is rejecting queries it used to accept". What actually
parses:

* Chinese inside a SPARQL **string literal** or a `#` **comment** parses; the
  grammar is Unicode.
* Chinese inside a **variable name or prefixed local name** is grammatical
  (the `PN_CHARS_BASE` ranges include CJK), so a translator who "translates"
  `?person` to `?人` produces a query that still round-trips — the result is a
  book whose examples no longer match the English ones, which is a
  consistency bug this test cannot see.
* A translated **keyword** or a full-width `｛` `（` `。` inside the block does
  not parse and consumes one unit of the ceiling.

`shipped_sparql_examples.rs` is broader (it also parses single-quoted shell
arguments in `sh`/`bash` fences and quoted string literals in prose that begin
with a SPARQL keyword), has no ceiling, and hard-fails on the first candidate
that does not parse. A Chinese page keeps the shell examples verbatim, so the
only new exposure is the prose pass, which keys on ASCII double quotes and
English keywords and therefore ignores Chinese quotation marks 「」“”.

With gettext, the translated blocks live in `.po` and **neither test sees
them**. That is safer for the ceiling and worse for correctness; the
translation gate in §4 must extract the translated fences and parse them.

### 3.6 `mdbook build` and search

No plugins: `book.toml` has `[book]` and `[output.html]` only, so nothing
in the build assumes English except mdBook itself. Local `mdbook` is 0.4.52;
`.github/workflows/docs.yaml` pins `MDBOOK_VERSION: 0.5.3` and installs the
musl tarball. Both were tested with a three-line `zh` book:

```
[book] language = "zh"
# 三元组项与具体化
本工具包实现 RDF 1.2 的三元组项（triple term）。PurRDF 支持 SPARQL 查询与 SHACL 验证。
```

| | 0.4.52 (local) | 0.5.3 (CI) |
|---|---|---|
| `<html lang>` | `zh` | `zh` |
| search pipeline | trimmer, stopWordFilter, stemmer — English | same, `lang: English` |
| CJK characters in the inverted index | **0** | **0** |
| `title` field root branches | none | none |
| Latin tokens indexed | `1.2`, `rdf`, `shacl`, `sparql`, `term）。purrdf`, `tripl` | same |

The shipped `elasticlunr.min.js` tokenizer is `defaultSeperator=/[\s\-]+/`;
the English trimmer then strips non-Latin characters, so every Chinese word
vanishes and full-width punctuation is left glued to Latin tokens
(`term）。purrdf`). The `language` key changes the `lang` attribute and
nothing else. Practical effect: a reader searching 三元组 gets nothing;
searching `SPARQL` still works because the house style keeps acronyms in
Latin. Options, in order of honesty: (1) disable search on the Chinese build
(`[output.html.search] enable = false`) and say so in the introduction; (2)
accept Latin-only search and say so; (3) post-process `searchindex.js` with a
CJK bigram tokenizer — a new build step and a fork of the searcher, out of
proportion for 0.14.0.

`docs.yaml` triggers on `paths: docs/book/**` and builds exactly
`mdbook build docs/book`, then folds the playground into the Pages artifact.
A second language output needs its own build line and its own fold; a
parallel tree outside `docs/book/` would not even trigger the PR build.

### 3.7 Generated and pinned content

* `docs/book/src/entailment-rules.md` is **emitted wholesale** by
  `cargo run -p purrdf-entail --example gen_rule_inventory` and byte-compared
  in `scripts/check-generated.sh` (`sync_file "$tmp/entailment-rules.md"
  docs/book/src/entailment-rules.md`). The generator carries about 145 words
  of English literal prose across 16 emit sites. A parallel-tree translation of
  that chapter would have to be regenerated by a Chinese-emitting generator or
  it drifts; gettext handles it naturally (the generated Markdown is the
  `msgid` source; a regeneration changes the `msgid`, the entry goes fuzzy,
  English shows through).
* `crates/rdf-capi/tests/abi.rs` asserts that `crates/rdf-capi/README.md`
  contains the literal ABI version string; a Chinese C-ABI README would need
  the same assertion or would silently go stale. (Recommended: do not translate
  it — §2.3.)
* `crates/purrdf/tests/readme_quickstart.rs` duplicates the root README's Rust
  quickstart verbatim as a test. The translated README keeps the same code
  block, so the test remains honest without change.
* No test pins `AGENTS.md` prose or a crate count as a bare number; `AGENTS.md`
  is read only by `check-doc-claims.py` for release-lane sentences, which stay
  English.

## 4. Tooling recommendation

### 4.1 The two options, against this repository

| Criterion | (a) `mdbook-i18n-helpers` (gettext `.po`) | (b) parallel `docs/book-zh/` tree |
|---|---|---|
| Availability | crates.io `0.4.0`; **not installed** here; nothing pins it | nothing needed |
| Pinning | must be pinned like `GIT_CLIFF_VERSION := 2.13.1` and `MDBOOK_VERSION: 0.5.3`; CI installs from a release tarball or `cargo install --locked --version` | none |
| Lag surfaced to the reader | **yes, per paragraph**: an untranslated or fuzzy `msgid` renders in English, so a reader sees exactly which paragraphs lag | **no**: a stale page reads as current unless a hand-maintained banner says otherwise |
| Lag surfaced to the maintainer | `msgmerge` marks fuzzy/obsolete entries; a CI step can count them and fail above a threshold | only by diffing commit dates by hand |
| Existing gates | **blind** — no gate scans `.po`; needs a new step that renders the translated Markdown to a temp tree and runs the four scripts plus the fence parser over it | apply automatically (brand casing, issue refs, spec attribution, claim sweeps, both SPARQL tests) — with the CJK blind spots of §3 |
| `SUMMARY.md` and chapter titles | translated as strings in the same `.po` | duplicated structure that drifts when a chapter moves |
| Generated `entailment-rules.md` | handled (regeneration → fuzzy → English fallback) | needs a Chinese-emitting generator |
| Build/CI cost | one preprocessor, one extra `mdbook build` with `MDBOOK_BOOK__LANGUAGE=zh-Hans`, one extra Pages folder; a normalisation pass (`mdbook-i18n-normalize`) on first adoption | one extra `mdbook build`; `docs.yaml` `paths` and fold lines |
| Translator ergonomics | standard PO editors; code blocks are extracted as messages too, so translators *can* alter them (mitigated by the fence-parse step above) | plain Markdown editing |
| Search | broken for CJK either way (§3.6) | same |

Recommendation: **(a)**, because the lead risk in §8 is a stale translation
that reads as current, and (a) is the only option that makes that state
visible without a process that someone has to remember. The price is a new
gate step (medium, §7 phase 0) and one more pinned tool.

### 4.2 README convention

GitHub does not select a README by the reader's locale; `README.zh-Hans.md` is
a plain file that the English README links to (a one-line language bar at the
top, 「简体中文」, is the common convention). Root `*.md` files are scanned by
all four scripts, so the Chinese README is gated the same way as the English
one. Keep it a full translation of the *preamble and quickstart*, not a
bilingual README: the 105-claim gate reads `README.md` sentences, and a
bilingual file would double the sentence count it has to find.

### 4.3 Registry READMEs

crates.io reads exactly one file per crate (`readme = "README.md"` in all 23
publishable manifests), PyPI reads exactly one (`readme = "README.md"` in
`pyproject.toml`), and npm renders the package's `README.md`. **None of the
three registries supports a per-language README.** The realistic move is a
single sentence near the top of the Python and npm READMEs linking to the
Chinese book, not a translated registry page.

## 5. Terminology, voice and glossary

### 5.1 What the house page settles

The house page fixes three things that this assessment does not need to
re-decide:

* **Brand invariance holds in Chinese.** `PurRDF`, `GMEOW`, `GTS`, `RDF 1.2`,
  `SPARQL`, `SHACL`, `OWL 2 DL`, `Rust`, `Python`, `WebAssembly/JavaScript`
  and `C` appear untranslated inside Chinese sentences, wrapped in Chinese
  classifiers: 「RDF 1.2 工具包」, 「SPARQL、SHACL 与图传输」, 「符合 OWL 2 DL
  的数字化存在超级词汇表」, 「LLM 驱动的变更日志自动化」. This is exactly the
  `docs/BRAND.md` rule, and it means the invariant token sits directly against
  CJK characters — which is why §3.1's spacing finding matters.
* **Term policy: acronyms and spec names stay English; concepts with a
  settled mainland rendering use it.** The page uses 本体 (ontology), 推理
  (reasoning), 规范化 (canonicalization), 图传输 (graph transport), 图原语
  (graph primitives), 声明溯源 (claim provenance), 内容寻址 (content-addressed),
  关联数据 (linked data), 机器学习, 微服务, 大数据. Its `/zh/purrdf/` page adds
  三元组 (triple), **三元组项 (triple term)**, 数据集 (dataset), 编解码器
  (codec), 验证 (validation). Where the house has already chosen, the book
  follows.
* **Typography:** full-width CJK punctuation (。，：；「」), a half-width space
  between Latin and CJK runs, code and identifiers in backticks
  (`` `did:web:blackcatinformatics.cn` ``).

Two further observations from the same page are evidence, not style: its
`/zh/purrdf/` page still carries one paragraph in English (the second
paragraph is the untranslated English tagline) — translation lag on the
company's own site, in the wild — and the term the house uses for
"reasoning-centred" is 「以推理为核心」, which settles 推理 for *reasoning* and
leaves *entailment* open (§5.3).

### 5.2 Register: how much of the voice carries where

The house page is formal with a deliberate classical flourish: third person
throughout, no 您/你, first person 吾辈, archaic vocabulary (桑梓, 泽被四海,
津梁, 阁下) in long parallel clauses, mixed with contemporary technical
vocabulary. A marketing page's register is not automatically right for
reference material, so the boundary has to be stated rather than assumed.

Recommended boundary, confirming the expectation in the brief:

| Element | Where it applies |
|---|---|
| Term-handling conventions (§5.1) | everywhere, exactly |
| Typography (§5.1) | everywhere, exactly |
| Formal register, third person, no 您/你 | everywhere |
| Classical flourish (吾辈, 津梁, 泽被四海, 阁下) | the book's `introduction.md` and the README preamble **only** — and even there, at most a sentence or two of framing |
| Plain formal technical Chinese | every concept page, the SPARQL reference, getting-started pages, CLI reference, the diagnostic-code reference, error text a developer scans under pressure |

The reason is the reader's task. A developer on `sparql/querying.md` is
looking for whether `GROUP_CONCAT` is order-stable; a sentence that makes them
parse 津梁 first has cost and no benefit. The introduction is the one place
where the reader is deciding whether to trust the project, which is what the
house voice is for. The 融通中西 framing belongs there and nowhere else in the
documentation.

One sentence rendered both ways, so the difference is visible:

> *English source (README):* "PurRDF keeps one engine and one behaviour, and
> carries it verbatim into Rust, Python, WebAssembly and C."
>
> *Introduction / README preamble (house flourish):*
> 「PurRDF 以一引擎、一行为为本，使同一张 RDF 1.2 图行于 Rust、Python、
> WebAssembly 与 C 之间而其义不移——此即吾辈所架之津梁。」
>
> *Reference page (plain formal technical):*
> 「PurRDF 以单一 Rust 引擎承载 RDF 1.2 图，并将同一行为原样带入 Python、
> WebAssembly 与 C。」

### 5.3 What W3C has translated, and what it does not cover

The W3C translation registry lists, for `zh-Hans`, exactly these
semantic-web documents, **all volunteer translations**: *RDF Primer* (RDF 1.0
era, 「RDF 入门」), *RDF Concepts and Abstract Syntax* (RDF 1.0,
「资源描述框架（RDF）：概念与抽象语法」), *OWL Web Ontology Language Overview*
and *Guide* (OWL 1), and *SPARQL 1.1 Overview* (「SPARQL 1.1 概述」). There is
**no** Chinese translation of RDF 1.1 or 1.2 Concepts, Turtle, TriG, JSON-LD,
SHACL, ShEx, SPARQL 1.1 Query itself, SPARQL 1.1 Entailment Regimes, or any
RDF 1.2 document. Every RDF 1.2 term — triple term, reifier, `rdf:reifies`,
annotation syntax, base direction — is therefore uncovered by W3C, and the
only established source is the mainland knowledge-graph (知识图谱)
literature, which settled the RDF 1.0/1.1 and OWL vocabulary but has had
nothing to say about 1.2.

### 5.4 Glossary sample: 30 load-bearing terms

Legend — **E**: established mainland rendering (use it); **H**: fixed by the
house page (use it); **C**: no established rendering, PurRDF coins one; **K**:
keep English, gloss on first use.

| # | Term | Rendering | Basis | Note |
|---:|---|---|---|---|
| 1 | triple | 三元组 | E, H | universal in 知识图谱 literature |
| 2 | quad | 四元组 | E | |
| 3 | named graph | 命名图 | E | |
| 4 | dataset | 数据集 | E, H | |
| 5 | blank node | 空节点 | E | |
| 6 | literal | 字面量 | E | |
| 7 | datatype | 数据类型 | E | |
| 8 | language tag | 语言标签 | E | |
| 9 | IRI | IRI | K | gloss 国际化资源标识符 once; never translate in running text |
| 10 | ontology | 本体 | E, H | |
| 11 | knowledge graph | 知识图谱 | E | the audience's own term for the field |
| 12 | reasoning / inference | 推理 | E, H | |
| 13 | entailment | 蕴涵 | E (logic) | 蕴含 also circulates; pick 蕴涵 and gate it |
| 14 | entailment regime | 蕴涵机制 | C | keep "entailment regime" in parentheses on first use; no W3C precedent |
| 15 | materialization | 物化 | E | database/KG usage |
| 16 | closure (inference) | 推理闭包 | E, qualified | bare 闭包 collides with the programming sense; always qualify |
| 17 | canonicalization | 规范化 | H | not 标准化; `RDFC-1.0` invariant |
| 18 | canonicalization profile | 规范化 profile | K + H | "profile" has no settled rendering (子集/概貌/配置 all circulate); keep English, identifier `purrdf-rdfc12` invariant |
| 19 | codec / serialization | 编解码器 / 序列化 | H, E | |
| 20 | round-trip | 往返（转换） | E | |
| 21 | determinism | 确定性 | E | |
| 22 | provenance | 溯源 | H | house: 声明溯源; not 出处/来源 |
| 23 | triple term (RDF 1.2) | 三元组项 | H | adopted by the house `/zh/purrdf/` page; W3C has none |
| 24 | reifier / `rdf:reifies` | 具体化节点 / `rdf:reifies` | C | 具体化 (reification) is established; the noun for the subject is coined; the IRI never changes |
| 25 | statement layer / annotation | 陈述层 / 注解 | C / E | 陈述 (statement) is established; 注解 for the `{| |}` annotation syntax |
| 26 | base direction (`--ltr`/`--rtl`) | 基础方向 | C, with precedent | W3C i18n articles in Chinese use 基础方向 for HTML/CSS base direction; the flags are invariant |
| 27 | property function | 属性函数 | E (Jena usage) | Chinese Jena material uses 属性函数; adopt |
| 28 | composite datatype (`SEP-0009`) | 复合数据类型 | E | natural DB rendering; the SEP number stays |
| 29 | governor / budget | governor（执行调控器）/ 预算 | K / E | no settled rendering for "governor" in this sense; 预算 is established for resource budgets |
| 30 | ledgered (`xfail` ledger) | 台账（预期失败台账） | E, qualified | 台账 is the engineering register word; 账本 reads as blockchain |
| 31 | walk vs. path | 游走 vs. 路径 | E | the pair maps cleanly (随机游走 is standard); keep the distinction the code makes |
| 32 | out-of-core | 核外 | E (HPC) | 核外计算 is standard |
| 33 | interned / interner | 驻留 / 驻留表 | E (Python community) | |
| 34 | fixpoint, semi-naive | 不动点, 半朴素 | E | |
| 35 | embedding, kNN, full-text search | 嵌入（向量嵌入）, k 近邻, 全文检索 | E | the audience's core vocabulary |
| 36 | slice | 切片 | E | |
| 37 | shape, focus node, validation report | 形状, 焦点节点, 验证报告 | E | SHACL Chinese usage |

Thirty-seven rows, because the seven RDF 1.2 and PurRDF-specific ones (14, 18,
24, 25, 26, 29, 30) are the ones that will be argued about, and the reader
should see them beside the settled ones. Roughly a third of the table is
**C** or **K**.

### 5.5 Policy for terms with no established rendering

Consistent with the house style and cheaper than a wrong coinage:

1. **First use on a page:** the coined Chinese term, then the English term in
   parentheses — 「蕴涵机制（entailment regime）」. Thereafter the Chinese term
   alone on that page.
2. **Where the English term is also an identifier** (`rdf:reifies`, `--ltr`,
   `purrdf-rdfc12`, a diagnostic code), the identifier appears in backticks
   and is never translated, glossed or not.
3. **Where no rendering is confident** (18, 29), keep the English word inside
   the Chinese sentence, as the house page does with `LLM` and `DevOps`, and
   gloss it once.
4. **The glossary is a file, not a page.** Put it at
   `docs/book/po/glossary-zh-Hans.md` (or beside the `.po`) and have the
   translation gate reject a `msgstr` that uses a rendering the glossary maps
   to a different term (e.g. 蕴含 where the glossary says 蕴涵, 标准化 where it
   says 规范化). A wrong or inconsistent rendering of a load-bearing term is
   worse than English with a gloss; the gate is what keeps ten translators
   from producing four words for one concept.

## 6. Distribution reality for a mainland reader

Access and latency only. GitHub Pages (the book), docs.rs, crates.io, PyPI and
npm are all intermittently slow or unreachable from the mainland; a reader who
cannot open the book cannot benefit from its translation.

* **crates.io:** developers routinely point Cargo at a mirror — TUNA
  (`mirrors.tuna.tsinghua.edu.cn/crates.io-index`), USTC
  (`mirrors.ustc.edu.cn/crates.io-index`) or `rsproxy.cn` — via
  `~/.cargo/config.toml` `[source]` replacement. PurRDF's crates are ordinary
  crates and mirror without any action on this side.
* **PyPI:** `pip install -i https://pypi.tuna.tsinghua.edu.cn/simple purrdf`
  (also USTC and Aliyun mirrors). Wheels mirror automatically.
* **npm:** `npmmirror.com` (the former cnpm/taobao registry) mirrors the whole
  registry; `npm config set registry https://registry.npmmirror.com`.
* **docs.rs and GitHub Pages:** no mainland mirror exists. The company already
  operates `blackcatinformatics.cn` (the house page's footer links it and its
  `did:web` uses it); serving the Chinese book from that domain is the direct
  remedy and needs only the built `book/zh-Hans/` artifact copied to it.
* **Git hosting:** Gitee is the usual mirror target for a repository that
  mainland contributors need to clone; a read-only mirror is a one-time setup.

Should the install docs mention this? A single paragraph in the Chinese
`getting-started` pages naming the three mirror families is low-cost and is
what a mainland reader expects to see; mirror *configuration* is the reader's
own environment and the docs should say so rather than script it. The English
pages need nothing.

## 7. Phased plan (magnitude relative to crate work in this repository)

Magnitudes: **small** ≈ a gate-script change with its self-test; **medium** ≈
a new conformance harness or a bindings feature; **large** ≈ a mid-sized
crate; **multi-release** ≈ larger than any single crate here.

| Phase | Content | Words in scope | Magnitude |
|---|---|---:|---|
| **0 — infrastructure and gate fixes** | pin `mdbook-i18n-helpers`; `mdbook-i18n-normalize` pass; `book.toml` preprocessor; `docs.yaml` second build and Pages fold; `blackcatinformatics.cn` publish step; new gate that renders translated Markdown and runs the four scripts plus the fence parser; glossary gate; `check-brand-casing.py` ASCII fix; `check-issue-refs.py` lookaround fix; Chinese `DISCLAIMERS`; `。！？` in `_TERMINATOR`; search disabled or declared Latin-only on the zh build; each tightening proven both ways | — | **medium** (six small gate changes plus one medium CI change) |
| **1 — entry path** | `README.zh-Hans.md`; `introduction.md`; `getting-started/{python,rust,javascript}.md`; `concepts/{interned-dataset,rdf12,codecs}.md`; `bindings/python/README.md`; `interop/rdflib.md`; the glossary itself; a mirror paragraph | ≈ 11,900 | **medium** |
| **2 — SPARQL extensions and retrieval** | `sparql/querying.md` whole (composite datatypes, statistical aggregates, path witnesses, extension hosts); `sparql/results.md`; `crates/text/README.md`; `crates/cdt/README.md`; `crates/geo/README.md`; `docs/design/purrdf-embedding-knn.md`; `docs/design/purrdf-text-scoring.md`; the reader-facing sections of `docs/PURREMB.md` | ≈ 24,000 | **large** — `querying.md` alone is a third of the book and holds 22 of the 24 gated SPARQL fences |
| **3 — Python API guide** | a Chinese API guide hosted in the book, derived from the 5,171 `#`-comment words of `__init__.pyi` and the 19,336 rustdoc words behind `__doc__`; **not** a second `.pyi` — Python, pyright and mypy have no locale-selected stub or docstring, so a parallel doc page is the only mechanism, and it drifts from the stub unless the gate diffs the two signature lists | ≈ 24,500 source words | **large** |
| **4 — remainder of the book** | `entailment.md`, `validation/{shacl,shex}.md`, `concepts/{projections,base-iris,jsonld,canonicalization,visualization}.md`, `datalog.md`, `gts.md`, `slices.md`, `interop/rdfjs.md`, `project/*`; `entailment-rules.md` via the gettext fallback; `docs/RDF12-CANON-PROFILE.md`, `docs/SPARQL-GOVERNOR-PROFILE.md`, `docs/COLUMNAR.md` | ≈ 27,000 | **large** |
| **5 — runtime strings** | a locale mechanism for clap help (none exists upstream), `RdfDiagnostic.message` (139 sites, tests pin messages), `Display` impls (269 sites), JSDoc; downstream matchers in `gmeow-ontology` and the playground; wasm-size-neutral but every binding grows a locale switch | ≈ 45,000 code-embedded words | **multi-release** — recommend *against* for 0.14.0; ship instead a Chinese reference page for the 79 diagnostic codes (documentation, phase 1 or 2, **small**) |
| **never** | rustdoc (873,106 words; docs.rs has no i18n); `GTS-SPEC.md` (normative, governed elsewhere); `CONFORMANCE.md` and `BENCHMARKS.md` (numbers); contributor docs | — | — |

Phases 1 and 2 are what an AI developer reads before deciding to use the
library; phase 0 is what stops phases 1 and 2 from rotting. Phase 4 before
phase 3 is defensible if translator capacity is prose-shaped rather than
API-shaped; the reverse order is recommended because the Python surface is
this audience's entry point.

## 8. Risks

1. **Translation lag masquerading as current documentation (lead risk).** The
   book changed in most recent releases; a Chinese page that was accurate at
   0.14.0 and silently wrong at 0.15.0 is worse than no page, because the
   reader has no signal. Only the gettext fallback surfaces this per paragraph
   by construction. Even with it, a *fuzzy* entry that a translator approves
   without re-reading is a stale paragraph again; the CI step must report the
   fuzzy and untranslated counts per release and the Chinese introduction must
   state which release the translation tracks. The house's own `/zh/purrdf/`
   page, with one paragraph still in English, is the shape of this risk.
2. **Gate blindness.** `.po` files are outside every scanner; glued CJK
   defeats the brand and process-token gates; the overclaim ban reads a
   Chinese paragraph as one sentence. Without phase 0 the translation is the
   first ungated prose in the repository.
3. **Search that returns nothing.** A Chinese reader's empty search result
   looks like "the book does not cover this" — a silent failure aimed at
   exactly the reader the translation is for. Disable or label it.
4. **Terminology drift.** Roughly a third of the glossary is coined. Without
   a glossary gate, different pages (or different releases) will render the
   same term differently, and the reader will conclude they are different
   concepts. This is the failure mode that makes a translation *worse* than
   English with a gloss.
5. **Gated numbers duplicated.** Every restated count in a translation is a
   count the 105-claim gate does not see. Keep scoreboards in English and
   link; where a number must appear in Chinese, either register a second
   sentence with the gate or accept that the fuzzy check is the only guard.
6. **Code blocks edited by translators.** A translated keyword breaks a fence
   and burns the 122 ceiling with a misleading "parser regression" message;
   a translated variable name round-trips fine and silently diverges from the
   English example. The phase-0 fence parser catches the first; only a
   byte-equality check of fenced blocks between `msgid` and `msgstr` catches
   the second, and it should be the default with an explicit per-block
   opt-out for comments.
7. **Register mismatch.** Classical flourish on a reference page reads as
   affectation to the target developer; plain register on the introduction
   reads as generic. The boundary in §5.2 has to be written into the
   translator brief, not left to taste.
8. **Reachability.** A translation the reader cannot load from the mainland
   has no audience. GitHub Pages alone is not enough; the `.cn` mirror is
   part of the deliverable, not an afterthought.
9. **Ownership.** A translation with no named owner per release lags by
   default. The release checklist in `docs/RELEASE.md` should gain a line
   ("zh-Hans `.po` merged, fuzzy count reported") or the lag risk in item 1
   is guaranteed rather than merely likely.
10. **Runtime-string temptation.** Once the book is Chinese, the first request
    will be Chinese error messages. Saying no for 0.14.0 — and saying why, in
    the Chinese introduction — is part of the plan.
