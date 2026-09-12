<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# First-party XSD/XPath `regExp` conformance corpus

Grades `purrdf_core::xsd_regex::compile` — the one shared translation from the
XSD/XPath regular-expression dialect onto the `regex` crate that `sh:pattern`,
SPARQL `REGEX`/`REPLACE` and ShEx `PATTERN` all route through.

The cases are **hand-derived** from the two normative texts, cited per file:

* *XML Schema Part 2: Datatypes* Appendix G, "Regular Expressions" — the
  `regExp` grammar, the multi-character escapes, and the block escapes;
* *XQuery and XPath Functions and Operators 3.1* §5.6 — `fn:matches`'s
  additions to that grammar (anchors, back-references) and §5.6.2's flags.

There is no redistributable W3C test suite for this surface: the XML Schema
datatype suite exercises `pattern` facets embedded in schema documents rather
than the regex language in isolation, and the SPARQL and SHACL suites test a
handful of patterns incidentally. The alternative to a first-party corpus is no
corpus, so this is first-party — and therefore carries **no xfail ledger**. A
first-party corpus that ledgers its own cases is a corpus that grades the
engine against whatever the engine happens to do.

## File format

One `.cases` file per construct group. Lines are UTF-8, `\n`-terminated:

* a line whose first non-space character is `#`, or which is blank, is a
  comment — use them to cite the clause each group comes from;
* every other line is exactly four TAB-separated fields:

  ```text
  pattern <TAB> flags <TAB> expectation <TAB> input
  ```

`expectation` is one of:

| value | meaning |
|---|---|
| `match` | the pattern compiles and `is_match(input)` is true |
| `nomatch` | the pattern compiles and `is_match(input)` is false |
| `error` | the pattern (or its flags) does not compile, and `input` is a substring the error message must contain |

`flags` is the XPath F&O §5.6.2 flag string, or `-` for none.

### Escapes

The `pattern` and `input` fields are **verbatim**, so a pattern reads exactly
as a shape author would write it — `\i`, `\p{IsGreek}` and `[a-z-[aeiou]]` all
appear as themselves. Only four sequences are decoded, and only because the
characters they name cannot be written literally in a tab-separated line:

| escape | code point |
|---|---|
| `\t` | U+0009 |
| `\n` | U+000A |
| `\r` | U+000D |
| `\uXXXX` | the code point with that 4-hex-digit value |

Every other `\`-sequence passes through unchanged, both characters. That rule
is what makes `\\uXXXX` — a regex escape for a literal backslash followed by
the letters `uXXXX` — decode correctly: the scanner sees `\` followed by `\`,
which is not one of the four, so it emits both and resumes at `u`.

A `\uXXXX` in a *pattern* is unambiguous because the XSD `regExp` grammar
defines no `\u` escape at all, so the sequence can never be part of a pattern
a caller would legitimately write.

The `input` field must be non-empty; the empty subject is covered by the unit
tests in `purrdf_core::xsd_regex` rather than here, because an empty final
field would be indistinguishable from a line with trailing whitespace stripped.

## Adding a case

Add the line, then bump the exact count in
`crates/rdf-core/tests/xsd_regex_conformance.rs`. The harness asserts that
count so a deleted file or a mistyped line reduces coverage loudly instead of
silently.
