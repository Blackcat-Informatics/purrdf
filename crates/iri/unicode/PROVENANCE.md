<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

# Provenance of the vendored Unicode Character Database

This directory is the workspace's one vendored copy of the Unicode Character
Database. `purrdf-iri` owns it because it is the lowest Unicode-aware crate in
the workspace.

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

From `https://www.unicode.org/Public/17.0.0/idna/`:

- `IdnaTestV2.txt`

## `16.0.0/` — Unicode 16.0.0

From `https://www.unicode.org/Public/16.0.0/ucd/`:

- `CaseFolding.txt` only, kept alongside the 17.0.0 copy so a case-fold
  differential between the two versions can be computed from the two
  published files rather than restated.
