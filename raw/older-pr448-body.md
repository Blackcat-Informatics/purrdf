# PR #448: rdf: add dated native XPath regex profiles and operational resource refusal

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Explicitly selected XPath 2.0 (2010-12-14) and XPath 3.1 (2017-03-21) patterns execute through one native compiler and matcher in SPARQL, SHACL and ShEx. Syntax findings remain language errors. Finite source, construction, matching and replacement limits return typed operational refusals before any partial answer. Backreferences, non-capturing groups and the quoted flag follow the selected dated law.

Native caches check profile, source, flags and current admission before reuse, and each request gets fresh execution limits. The existing Unicode generator supplies the category tables and direct full-case variants. Compatibility entry points and their public cache types keep their existing behavior. JSON Schema keeps its ECMA engine.

Closes #406

Implementation plan: https://github.com/Blackcat-Informatics/purrdf/issues/406#issuecomment-5988011853

## Changes in this finishing pass

- **Miri lane.** Two deep-stack witnesses had names containing `small`. That put them in the Miri SmallVec filter, and the hosted Miri job hit its 30-minute ceiling. They are renamed, and the shared Makefile filter is unchanged. `cargo test -p purrdf-core small -- --list` now lists the 18 SmallVec tests again.
- **XSD 1.0 Second Edition block names.** Both F&O 2.0 2E (7.6.1) and F&O 3.1 (5.6.1) say their syntax and semantics are those of XML Schema Part 2 Second Edition plus additions. That edition's Appendix F table therefore applies to both laws. It includes `Greek`, `PrivateUse` and `CombiningMarksforSymbols`, which the native compiler used to reject as syntax errors.
  - A name in both tables keeps its dated extent under 2.0 and its Unicode 17 extent under 3.1. Four names differ: `Specials`, `HangulSyllables`, `CJKUnifiedIdeographsExtensionA` and `ArabicPresentationForms-B`.
  - A name in neither table is FORX0002, as F&O 3.1 5.6.1.5 requires.
  - Vectors cover both profiles with valid and invalid neighbours. CONFORMANCE.md records the policy.
- **CompileSteps boundary.** An exact required / required-1 pair, found by bisection, for three pattern shapes.
- **No-op replacement.** `replace_all` with no match returns the input borrowed (F&O 3.1 5.6.4 Notes), so it no longer charges `OutputBytes`. Its neighbour with one match is still refused one byte below its exact requirement.
- **Ordinary large input at `Limits::new()`.** Each item below used to be refused at the defaults:
  - `.*` and `[a-z]+` over 130,000 characters, which needed one pending state per iteration.
  - An 8 MiB literal search (6 steps per position).
  - `.*needle.*`, which was quadratic.
  - A 30,000-character literal pattern, which needed 3 nodes and 20 construction cells per byte.

  The fixes:
  - A greedy single-character repetition is scanned once and keeps one pending state for all its stops. It skips stops that a following character atom cannot continue from.
  - A search skips start positions that cannot begin a match: first-character sets (literal unions as sorted ranges, case variants included), non-multiline `^`, and the starts that a failed leading unbounded run already covered.
  - The program, construction and step defaults are recalibrated so that every source admitted by the 64 KiB source bound compiles. The step bound is now 100,000,000.

  Exponential backtracking over 40 characters still refuses, and its matching neighbour succeeds. Differential tests compare every shortcut with the general path over all words of up to five characters.

### Bench (`crates/rdf-core/benches/xsd_regex.rs`, group `native_xpath_large`)

Both columns come from the same bench binary, pinned with `taskset`; "before" has the matcher shortcuts disabled.

| case | before | after |
|---|---|---|
| `run/32k` (`[a-z]+`) | 4.78 ms | 0.67 ms |
| `general/32k` (`([a-z])+`, general path, control) | 5.73 ms | 6.16 ms |
| `search/literal` (1 MiB, absent) | 172 ms | 17.1 ms |
| `search/choice` | 432 ms | 57.6 ms |
| `search/anchored` | 166 ms | 217 ns |
| `search/leading_run` (`.*needle`) | refused (MatchSteps) | 72.4 ms |
| `run_128k` | refused (MatchStates) | admitted |
| `compile_literal_30k` | refused at old defaults | 1.59 ms |

## Criteria (issue body and plan contract)

| # | Criterion | Status | Evidence |
|---|---|---|---|
| 1 | Explicit dated profiles; sibling API | MET | `xpath/mod.rs` `Profile`, `Profile::ALL`, `Profile::from_name`; `compile.rs` `compile` |
| 2 | Backreferences (both), `(?:` and `q` (3.1 only) | MET | `scan.rs`, `match.rs`; oracle tests (38,220 comparisons) |
| 3 | Complete applicable productions per dated law | MET | XSD 1.0 2E block table (`dated_blocks.rs`, test `xsd10_second_edition_block_names_are_part_of_both_dated_laws`); XML 1.0 2E `\i`/`\c` classes in both laws (`dated_names.rs`, test `name_escapes_use_the_xml_10_second_edition_classes_in_both_laws`, covering U+10000, U+0D7A and U+9FA6) |
| 4 | Flags s m i x q | MET | `x`: F&O 2.0 2E 7.6.1.1 and 3.1 5.6.2 state the same class-exempt rule |
| 5 | Ordered matching, captures, `$N`, FORX0003/FORX0004 | MET | `replace.rs`; no-match replacement is borrowed with no output charge |
| 6 | Finite VM limits, each boundary with a valid neighbour | MET | all 8 resources, including the CompileSteps pair; production-default tests for ordinary large input and adversarial refusal |
| 7 | Oversized pattern is a resource refusal before materialization | MET | `admit_pattern`; source bound + 1 byte refused on every surface |
| 8 | Operational errors through SPARQL/SHACL/ShEx | MET | host native suites. EXPLAIN now honours the per-request law (`explain_evaluates_under_the_requests_dated_law`), and the change path, rules and node expressions have selected-law doors |
| 9 | Unicode data from the existing generator only | MET | `unicode_tables.rs` is generated. The dated block and name tables are the W3C Recommendations' own enumerations, not UCD data |
| 10 | JSON Schema unchanged | MET | untouched |
| 11 | Additive API only | MET | new functions, twins and keywords only. The C ABI keeps existing signatures and adds status 13 at the end. `cargo semver-checks` passed for the library crates at the previous head |
| 12 | Gates | MET (hosted) | Per the owner's rule, local runs are targeted: tests, clippy `-D warnings`, fmt, the wasm32 builds and the hygiene scripts for every touched crate. Hosted CI is the full gate |
| 13 | PR / review / CI | see CI | hosted CI on `36a13ac79`. CodeRabbit has not reviewed the final head: its re-request was refused by the organization's usage spending cap |

### Profile reach

The selector takes a law's exact dated name: `xpath-2.0-2010-12-14` or `xpath-3.1-2017-03-21`. Every surface decodes it through `purrdf_validate::xpath_regex::parse_profile`, and an unknown name is refused with the accepted names.

- **CLI:** `--xpath-regex` on `query`, `update`, `validate` (all routes, including `--changes`), `shex`, `rules` and `node-expr`.
- **Python:** an `xpath_regex=` keyword on every SPARQL, SHACL (validate, prepared, change set, entail, apply_rules, eval_node_expr) and ShEx door, plus `purrdf.XPATH_REGEX_PROFILES`. The `.pyi` is updated.
- **WASM/JS:** an `xpathRegex` option on every SPARQL and SHACL export and their async twins, and on the Cloudflare handler. The `.d.ts` is updated.
- **C ABI:** fourteen `*_xpath_regex` twins taking a nullable law name, and the new `PURRDF_STATUS_REGEX_RESOURCE_ERROR = 13`.

Each surface has end-to-end tests:
- each law decides a law-distinguishing pattern: `(?:a)b` is valid only under 3.1, and `^(a)\1$` matches under both native laws but is refused by compatibility;
- a resource refusal sits beside its admitted neighbour at the source bound;
- unknown names (`xpath-3.1`, `XPATH-3.1-2017-03-21`, the empty string) are refused, while the exact names are accepted.

SHACL lint takes no law: it never compiles or matches a user pattern. ShEx is not exported on WASM or the C ABI.

W3C QT3: no QT3 `fn-matches` or `fn-replace` cases are vendored here, and none are added.

## Adversary findings F1-F6

- **F1, over-refusal of ordinary linear patterns.** Fixed with a linear-time thread machine (`crates/rdf-core/src/xsd_regex/xpath/pike.rs`) for programs without backreferences. The thread machine reports the backtracker's matches and captures, keeping one thread per distinct control state at each position. Group repetition no longer keeps a pending state per iteration. Backreference programs keep the backtracker. `MatchSteps` is now 250,000,000.
  - These shapes now answer at `Limits::new()` like the compatibility engine, at 1 MiB and 4 MiB in core and at production defaults through SPARQL, SHACL, ShEx and the CLI: `node.*graph.*zzz`, `alpha.*zzz`, `^([a-z]+ ?)+$`, `^(a|b)*$`, `^(ab)*$`, `^(?:ab)*$`, `^(\w+\s)*\w+$` and `^(a|aa)*$|^(a*)*b$`.
  - A differential test compares the thread machine with the backtracker over 75 patterns, four flag sets and every start position.
  - Backreference blowups and huge unanchored counted repeats still refuse, each beside an answered neighbour.
  - Rerunning the adversary's CLI probe turned every finding from rc=1 into the compatibility answer. On the bench, `search/leading_run` went from 69 ms to 24 ms and `run_128k` from 2.5 ms to 0.83 ms; the new adversary rows over 1 MiB, which the old matcher refused, take 39-338 ms.
- **F2.** WASM SHACL errors (sync and async), and Python SHACL, ShEx and SPARQL exceptions, now carry the `Resource::code()` identity (`code`/`errorCode`; `message_id`/`presentation`). Tests check this next to valid neighbours.
- **F3.** The CHANGELOG [Unreleased] Added section has entries for the Rust API, the CLI flag, Python, JS, the 14 C functions and status 13.
- **F4.** The book has the `xpath-*` diagnostic family table and law-selection sections in the SPARQL, SHACL and ShEx pages. The zh-Hans catalogue was regenerated with the book tooling and translated; `make check-i18n` reports 0 fuzzy and 0 untranslated.
- **F5.** The semver comparison at the final head passes:
  - purrdf-validate, through `cargo semver-checks --current-rustdoc/--baseline-rustdoc` against origin/main rustdoc JSON (the wrapper blocks the `--baseline-rev` path for this crate): 223 pass, 31 skip, no semver update required.
  - purrdf-core, -lex, -shapes, -shex and -sparql-eval, through `--baseline-rev origin/main`: 223 pass, 31 skip each.
- **F6.** `dated_names.rs` and `dated_blocks.rs` are now emitted by the existing table generator. Its inputs are verbatim, digest-frozen copies of XML 1.0 2E and XSD 1.0 2E under `vectors/w3c-recs/`, and `scripts/check-generated.sh` checks the output.

origin/main (`ce3c07192`, including the `RdfTriple` Drop change) is merged. `cargo check --workspace --all-targets` is clean, with no E0509 sites.

### Instruction counts (`perf stat -e instructions:u`, pinned to one CPU, median of 5, per call)

| case | before the thread machine | thread machine | final head |
|---|---|---|---|
| `^[a-z0-9]+$` on `"abc123"` | 4,916 | 5,456 | 2,528 |
| `needle` in `"hay needle stack"` | 3,197 | 2,780 | 2,642 |
| `needle` over 1 MiB of filler, absent | 111,682,479 | 105,107,901 | 100,436,072 |

The `plain_ascii` wall-clock rise (225 to 490 ns) was a real 11% instruction rise. It is fixed:
- class unions are flattened into sorted ranges at compile time;
- the abandonment allowance is no longer recomputed at every step.

`search/literal` never regressed in instructions; the wall-clock rise was host load. origin/main has no native XPath matcher, so the baseline is the branch's own pre-thread-machine revision.

origin/main `0d6575a46` is merged. `zh-Hans.po` was resolved with the book tooling, keeping both sides' translations; `make check-i18n` reports 2,989 translated, 0 fuzzy, 0 untranslated.

## Adversary pass 2 (N1-N3)

- **N1, counted repetition.** A set machine (`xpath/sets.rs`) takes over an abandoned backreference-free search. It holds a repetition's live counts as an ordered interval set per program point, so cost per position does not depend on how many counts are live. A cached reverse scan guides the capture pass for `find`/`replace` as a single thread.
  - Every pass-2 shape answers at `Limits::new()` like the compatibility engine, in core and through SPARQL, SHACL, ShEx, the CLI, C, WASM and Python: `(ab){1,1000}c`, `(ab){2,50}c`, `(ab){1,100}c`, `(ab|cd){1,20}e`, `((a|b){3}){5,9}c`, `((a|b){2}){2,5}c`, `(a|b){1,30}c`, `(a|b){3,9}c`, `(\w+\s){3,5}zzz` and `node.*graph.*zzz`. All adversary rows answer at 64 MiB, at about 3 steps per byte.
  - Bench rows that used to be 1.1-2.3 s refusals now take 1-77 ms.
  - Instruction counts on `plain_ascii`, a literal hit and a 1 MiB absent literal are unchanged: 2,528 / 2,640 / 100,436,042.
- **N2.** `(ab){1,100000}c` and `(a?){18446744073709551616}` now assert their answers (no match, and a match). Refusal pins remain only for backreference programs.
- **N3.** The `xpath-match-steps` row names backreference blowups. The Chinese Python README has the `xpath-*` `message_id` clause.
- **`Limits::new()` doc.** It now gives measured figures: about 1.0 step per byte on counted shapes and about 3 steps per byte on whole-input group matches; 3-11 ns per step; a backreference blowup is refused after about 3 s.

Accepted as the issue's intended operational resource bound (finite and deterministic), and documented with measured figures in the book's `xpath-*` diagnostic section:
- nested counted repetitions whose bodies can both repeat empty, with counts near the storage bound, e.g. `((a?){100000}){100000}`; `is_match` answers `((a?){1000}){1000}b`, but `find` over 100 KB does not;
- `find` with a large constant cost: `(|a){18446744073709551616}b` at about 250 steps per byte of its match, answered up to about 1 MiB of match; `(a|aa){1,100000}b` at about 98 steps per byte, up to about 2.5 MiB;
- input past about 74-83 MiB, where the step bound itself is reached.

## Review status

CodeRabbit has not reviewed the final head. A re-request was refused by the organization's usage spending cap. No review threads are open.

