<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored W3C Recommendations behind the dated XPath regex tables

Both dated XPath regular-expression laws (F&O 2.0 Second Edition section 7.6.1
and F&O 3.1 section 5.6.1) take their syntax and semantics from XML Schema
Part 2 Second Edition, whose `\i` and `\c` escapes are defined by XML 1.0
Second Edition's Appendix B classes. `purrdf-core` holds those classes and
the Schema edition's own block-name table as two generated files:

| Generated file | Table set | Read from |
|---|---|---|
| `crates/rdf-core/src/xsd_regex/xpath/dated_names.rs` | `xpath-dated-names` | productions \[4\] and \[84\] to \[89\] of `REC-xml-20001006.html` Appendix B, with the `\i` and `\c` rows of `REC-xmlschema-2-20041028.html` Appendix F.1 |
| `crates/rdf-core/src/xsd_regex/xpath/dated_blocks.rs` | `xpath-dated-blocks` | the block table after production \[36\] of `REC-xmlschema-2-20041028.html` Appendix F.1 |

Each is the stdout of
`cargo run -p purrdf-lex --example gen_unicode_tables --locked -- <table set>`,
and `scripts/check-generated.sh` diffs it against the committed file. Neither
table is Unicode Character Database data, so neither is held to the
workspace's Unicode version.

The two Recommendations are vendored whole and verbatim, not excerpted, so
each copy can be checked against its published URL by digest and carries its
own copyright notice, status section and licence links, as the W3C document
licence requires of every copy. Every file here is held byte-frozen by
`scripts/check-corpus-frozen.py` against
`scripts/conformance-frozen/vectors-w3c-recs.sha256`; `.gitattributes` keeps
git from converting their line ends (`REC-xml-20001006.html` uses CRLF).
Fetched 2026-10-06; each digest below equals that of a copy fetched on
2026-10-05.

- `REC-xml-20001006.html`: <https://www.w3.org/TR/2000/REC-xml-20001006>,
  *Extensible Markup Language (XML) 1.0 (Second Edition)*, W3C Recommendation
  6 October 2000, served as `REC-xml-20001006.html` (`text/html;
  charset=iso-8859-1`, last modified 2018-10-09; the content is ASCII).
  196,460 bytes, SHA-256
  `e5350fb462ada6babfc43d72263a955bddf78605180457c6e006886f60a89b67`.
- `REC-xmlschema-2-20041028.html`:
  <https://www.w3.org/TR/2004/REC-xmlschema-2-20041028/>, *XML Schema Part 2:
  Datatypes Second Edition*, W3C Recommendation 28 October 2004, served as
  `datatypes.html` (last modified 2018-10-09; the content is ASCII). 644,011
  bytes, SHA-256
  `980de872aa2c50013d5202176eb85eef3f183a7df635fdcb6b4aa34c1c12eb72`.

## Licence

Copyright © 2000 and © 2004 W3C® (MIT, INRIA, Keio; MIT, ERCIM, Keio), All
Rights Reserved. Both documents are vendored under the W3C document licence
their copyright notices link to; they are not relicensed, and they are not
part of any published package.

- `REC-xml-20001006.html` links
  <https://www.w3.org/Consortium/Legal/copyright-documents-19990405>, vendored
  verbatim here as `LICENSE-W3C-document-19990405.html` (*W3C Document Notice
  and License*, 4,714 bytes, ISO-8859-1, last modified 2002-02-13), SHA-256
  `bfc01b4569ee865dac7dc6653a6df012605e8a9212fa5c55bcda68f53b8936fc`.
- `REC-xmlschema-2-20041028.html` links
  <https://www.w3.org/Consortium/Legal/copyright-documents>, which resolves to
  the 2023 document licence recorded in `LICENSES/W3C-document-license.html`
  (see `LICENSES/PROVENANCE.md`).

`license-inventory.toml` registers the two documents as the `w3c-xml10-2e`
and `w3c-xmlschema2-2e` materials, each with the notice it links.
The generated tables are first-party source: they hold the code points the
Recommendations enumerate, and no text of either document.
