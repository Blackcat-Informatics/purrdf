# Native glossary gate: Task 1

Status: the initial review's multiline-inline correction is implemented and all
source-dependent focused reruns PASS. Source is ready for independent re-review;
the initial BLOCK is not treated as cleared without that independent result.
Base origin/main `6273b6173f3b3d98f49629a162ba88da8bc26477`.
Source is uncommitted. Root owns review, normal hooks, commit/push and issue
progress. This task performed no forge operation or Git mutation.

## Implementation

The review correction classifies block/definition boundaries first, then balances
inline syntax across each paragraph's soft line breaks. Blank lines, fences and
reference definitions end a segment; an unclosed destination cannot hide later
outside prose. Inline scanning skips already masked destinations/titles, so their
brackets/backticks cannot consume unrelated text. New native and default
production controls cover multiline links/images/labels/titles, source/translated
K URLs, hidden rejection prose, outside refusals, malformed links across blank
paragraphs, literal fenced URLs and exact byte/newline offsets. No documentation
scope cut substitutes for the correction. All source-dependent reruns below pass.
Exact backtick spans inside labels also preserve literal closing brackets and
visible code identifiers while their destinations stay hidden.

One `helper-census --glossary-gate` mode accepts `--root`, `--po`, `--glossary`,
`--self-test` and optional `--inventory FILE`. The selected glossary supplies
every rule/control; the root's default catalogue always supplies real
standardized-spelling specimens, independently of an external scan catalogue.
All default controls precede every normal scan. Missing/malformed inputs and
absent required specimens hard-fail.

Original Rust table/PO readers preserve seven columns, escaped pipes and regex
backslashes/backticks, GLOBAL reasons, K anchors, continuation strings, contexts,
header suppression, first-plural fallback, four escape decodings, unknown-escape
spelling and fuzzy/obsolete/empty suppression. Malformed strings/fields/keywords
fail. Only an unpublished host-tool first-party `purrdf-jsonschema` dependency
was added, with its layer edge and Cargo-generated lock entry. Public
`ecma::compile` and its bounded matcher remain the sole pattern engine: no
external/shipping dependency, feature or second matcher was added.

The compatibility boundary pins Unicode17 word/decimal properties, Python
whitespace, LF-only dot and terminal-LF dollar behavior, word boundaries,
Python3.13 empty-input `\B` and the four additional ASCII-ignorecase letters.
Word membership does not widen under ignorecase (U+0345 regression). Range
expansion asks the existing compiler about class membership rather than changing
literal `a-z` text. Fixed literal/class lookbehind is admitted; unsupported
escapes/modifiers/grouped or variable-width lookbehind, malformed classes and
resource exhaustion hard-fail. Diagnostics retain actual source labels and
selected glossary path/row/rule. No exact matched-span claim is invented.

One offset-preserving Markdown surface keeps labels, image alt labels, reference
labels, autolinks and code identifiers; masks balanced destinations/titles and
reference-definition lines without joining fragments; additionally masks exact
inline code and matching fenced blocks for rejection prose. Whole-document fence
state and actual line diagnostics are preserved. Its bounded contract is
documented, with no full CommonMark/HTML renderer claim.

Every current anchor is compiled and independently exercised with positive and
negative production-path compatibility specimens. All24 actual K tokens receive
isolated drop/keep/prefix/suffix/case/punctuation/CJK/code/link/title/definition
controls on both sides, retaining other active K obligations. ANY unrelated
offence fails an isolation fixture. Existing rejection/neighbour, cross-row,
global, inactive PO, concrete collision and real standardized-spelling controls
remain active. The narrow half-width Research Object English-gloss exception is
documented; both full-width forms and wrong bare/translated forms are tested.

Tracked Markdown selection remains git NUL-safe, byte-sorted, content-based at
15percent, with raw Unix filename bytes retained and the glossary excluded.
Whole translated documents receive GLOBAL refusals only.

## Actual current sweep

Native-generated `glossary-sweep.tsv` records2,956 active PO units plus1,225 lines
in3 selected Markdown documents, totalling4,181 translated units, under69 rows,
43 rejected renderings and24 K tokens. Every actual anchored paragraph was
scanned: zero wrong renderings and zero raw rejected-specimen occurrences in
visible Chinese prose. No collision context needed an exemption or translation
repair; no catalogue edit or broad exemption was made.

All41 applicable real-paragraph poison controls are refused by the production
evaluator using actual retained English/Chinese pairs with injected bad spelling,
without catalogue mutation. Knowledge graph and Research Object have zero active
anchored PO units: their two real-poison counts are honestly0, with synthetic
production controls still active. No current corpus coverage is claimed for them.

SHA256 identities:

- Catalogue: `2957999791d23c2f63fca754e208a5f1e9551e3c8318c6e4beffd12cb867e292`.
- Glossary: `bd35410c123b0ab9aa99033d02f3aea4d5fbd115562a1e9db21dad1a56e91428`.
- Inventory: `dbcfa444298a083b8d57acfdb3cfc47289aff65546716d7ffc4e5c09a15d9c79`.

## Settled focused checks after the review correction

Compiler `rustc 1.100.0-nightly (4b6d04e70 2026-09-13)`. All Cargo child builds
used `CARGO_BUILD_JOBS=8` and `--jobs 8` under root's shared-build-lane admission.

- `cargo test -p helper-census --jobs 8 --locked glossary:: -- --nocapture`:
  PASS,10 native tests; unrelated package tests filtered, not claimed run.
- `cargo clippy -p helper-census --all-targets --jobs 8 --locked -- -D warnings`:
  PASS on settled source.
- `cargo run -p helper-census --jobs 8 --locked -- --glossary-gate --self-test`:
  PASS,1,716 production-path controls.
- Same native command with
  `--inventory .stage/zh-hans-glossary-gate-harden-the/glossary-sweep.tsv`:
  PASS, actual4,181-unit scan and41 real-paragraph poison refusals.
- `python3 scripts/check-layers.py`: PASS,208 normal edges/42 members and6
  unsafe-code-forbidden crates.
- Existing still-wired `python3 scripts/check-i18n-glossary.py`: PASS with the
  same69 rows/43 rejections/24 tokens/4,181 units/3 files, preserving the interim
  caller. This does not substitute for native checks.
- `cargo fmt -p helper-census --check` and `git diff --check`: PASS.

Actual command/result logs are retained in this tasks directory:
`T1-focused-tests.log`, `T1-clippy.log`, `T1-self-test.log`, `T1-sweep.log`,
`T1-hygiene.log` and `T1-legacy-caller.log`. `T1-source-receipt.sha256` binds
all nine source files plus the catalogue and regenerated inventory. The new
paragraph projection does not change this catalogue's measured row/anchor/raw/
poison counts; the inventory was regenerated, not assumed unchanged.

Development failures were fixed: specimen derivation counted parentheses inside
classes, one Research Object poison accidentally formed an accepted gloss with
retained K text, and clippy found formatting allocations/Unicode spelling/style
issues. Subsequent audit fixed ignorecase word widening and literal range
rewriting. No rule was weakened and no verification bypass used. The settled
checks above passed after those corrections and the independent review's
multiline-inline correction. No source changed during or after this settled batch.

## Exact scope and remaining delivery

Modified `Cargo.lock`, `crates/helper-census/Cargo.toml`,
`crates/helper-census/src/main.rs`, `layers.toml` and
`docs/book/po/glossary-zh-Hans.md`. Added
`crates/helper-census/src/glossary/{mod,pattern,po,surface}.rs`.
No generated projection is affected by this host-only graph edge; none was
hand-edited. Normal generator/gates remain in settled qualification.

Task2 still switches Make/CI/staged snapshot callers coherently, retires old
Python and exercises actual external-path and index/working-tree poison
inversions. The old gate remains wired until that switch. Task3 still owns
full `make check`, i18n/render prerequisites and acceptance, hosted CI,
completion audit, PR and ghprsq integration. None of those is claimed here.
