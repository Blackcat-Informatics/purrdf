<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

# Provenance of the vendored Unicode Character Database

This directory is the workspace's vendored copy of the Unicode Character
Database at its current version. Every Unicode table in the workspace but one
is generated from it by `crates/lex/examples/gen_unicode_tables.rs`, at the
version `purrdf_lex::unicode::UNICODE_VERSION` names; the generator refuses a
file of another release. It is not the only vendored UCD file:
`purrdf-core` keeps its own `Blocks.txt` at Unicode 16.0.0
(`crates/rdf-core/vendor/unicode/`), pinned to the version of the Unicode
tables embedded in the locked `regex-syntax`, which its XSD regular-expression
block escapes must agree with.

Every file is vendored verbatim, carries the Unicode-3.0 licence declared in
`REUSE.toml` (text in `LICENSES/Unicode-3.0.txt`), and is held byte-frozen by
`scripts/check-corpus-frozen.py` against
`scripts/conformance-frozen/iri-unicode.sha256`. Fetched 2026-09-26.

## `17.0.0/` — Unicode 17.0.0

From `https://www.unicode.org/Public/17.0.0/ucd/`:

- `CaseFolding.txt`, `UnicodeData.txt`, `CompositionExclusions.txt`,
  `DerivedCoreProperties.txt`, `DerivedNormalizationProps.txt`,
  `NormalizationTest.txt`, `Scripts.txt`
- `auxiliary/WordBreakProperty.txt`, `auxiliary/WordBreakTest.txt`
- `emoji/emoji-data.txt`
- `extracted/DerivedJoiningType.txt`, `extracted/DerivedBidiClass.txt`

Added 2026-09-27, from the same directory, for the RFC 5892 derived property
(Join_Control, Noncharacter_Code_Point, White_Space; Hangul_Syllable_Type; the
three IgnorableBlocks):

- `PropList.txt` — `https://www.unicode.org/Public/17.0.0/ucd/PropList.txt`,
  header `PropList-17.0.0.txt`, dated 2025-06-30, SHA-256
  `130dcddcaadaf071008bdfce1e7743e04fdfbc910886f017d9f9ac931d8c64dd`
- `HangulSyllableType.txt` —
  `https://www.unicode.org/Public/17.0.0/ucd/HangulSyllableType.txt`, header
  `HangulSyllableType-17.0.0.txt`, dated 2025-01-27, SHA-256
  `5a57450afde0d082bc5026f7458649eac3b615490cc7e3d916b0367f1593c0e3`
- `Blocks.txt` — `https://www.unicode.org/Public/17.0.0/ucd/Blocks.txt`, header
  `Blocks-17.0.0.txt`, dated 2025-08-01, SHA-256
  `c0edefaf1a19771e830a82735472716af6bf3c3975f6c2a23ffbe2580fbbcb15`

Added 2026-09-27, from the same directory, for the property names and values
an ECMA-262 `\p{…}` escape accepts (`purrdf-jsonschema`'s generated
`crates/jsonschema/src/ecma/property_tables.rs`):

- `PropertyValueAliases.txt` —
  `https://www.unicode.org/Public/17.0.0/ucd/PropertyValueAliases.txt`, header
  `PropertyValueAliases-17.0.0.txt`, dated 2025-06-30, SHA-256
  `64e9a5f76f7a1e8b5a47d6a1f9a26522a251208f5276bdfa1559dac7cf2e827a`
- `PropertyAliases.txt` —
  `https://www.unicode.org/Public/17.0.0/ucd/PropertyAliases.txt`, header
  `PropertyAliases-17.0.0.txt`, dated 2025-04-25, SHA-256
  `4441f573caf952ffece1d7c892e7715bd7136dfc26f96eb6f268bf1e474715fb`

From `https://www.unicode.org/Public/17.0.0/idna/`:

- `IdnaTestV2.txt`

Added 2026-10-04, verbatim Unicode 17 data for extended grapheme boundaries
and finite emoji recognition:

- `auxiliary/GraphemeBreakProperty.txt` — SHA-256
  `d6b51d1d2ae5c33b451b7ed994b48f1f4dc62b2272a5831e7fd418514a6bae89`
- `auxiliary/GraphemeBreakTest.txt` — SHA-256
  `e2d134d2c52919bace503ebb6a551c1855fe1a1faec18478c78fff254a1793ec`
- `emoji/emoji-variation-sequences.txt` — SHA-256
  `bb3d09ef03f206012c7532dd52dc0a21c9efddba0135ea4cf0d9201b8b9bba7e`
- `https://www.unicode.org/Public/17.0.0/emoji/emoji-test.txt` — SHA-256
  `1d8a944f88d7952f7ef7c5167fef3c67995bcae24543949710231b03a201acda`

Added 2026-10-05 for XPath's full lower/upper case-variant relation:

- `https://www.unicode.org/Public/17.0.0/ucd/SpecialCasing.txt`, verbatim
  Unicode 17.0.0 supplement to `UnicodeData.txt`, header
  `SpecialCasing-17.0.0.txt`, dated 2025-07-31, SHA-256
  `efc25faf19de21b92c1194c111c932e03d2a5eaf18194e33f1156e96de4c9588`.

## `16.0.0/` — Unicode 16.0.0

From `https://www.unicode.org/Public/16.0.0/ucd/`:

- `CaseFolding.txt` only, kept alongside the 17.0.0 copy so a case-fold
  differential between the two versions can be computed from the two
  published files rather than restated.

Added 2026-10-09 for the admitted native unselected regex compatibility law:

- `UnicodeData.txt` — verbatim
  `https://www.unicode.org/Public/16.0.0/ucd/UnicodeData.txt`, SHA-256
  `ff58e5823bd095166564a006e47d111130813dcf8bf234ef79fa51a870edb48f`,
  BLAKE3 `24dd932e1b587f076f3895081f4eb2fd41c77881b3d84a2f743157f6b3f96c40`.

The existing Rust generator's `xpath-compatibility` mode derives Unicode 16
categories and simple-fold classes from these two exact original input
identities. They preserve the unselected evaluator law of locked
`regex-syntax` 0.8.11, while explicitly selected dated XPath laws continue to
use their Unicode 17 full-case relation. The shared XML terminal and Unicode
16 block table homes are reused. No external implementation is copied.
